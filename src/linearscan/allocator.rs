use extra::sort::quick_sort;
use extra::smallintmap::SmallIntMap;
use std::{vec, uint};
use linearscan::graph::{Graph, KindHelper, Interval,
                        IntervalId, InstrId, RegisterId, StackId, BlockId,
                        UseAny, UseRegister, UseFixed, GroupId,
                        Value, RegisterVal, StackVal};
use linearscan::flatten::Flatten;
use linearscan::liveness::Liveness;
use linearscan::gap::GapResolver;

pub struct Config {
  register_groups: ~[RegisterId]
}

pub struct AllocatorResult {
  spill_count: ~[StackId]
}

struct GroupResult {
  spill_count: StackId
}

struct AllocatorState {
  config: Config,
  group: GroupId,
  register_count: RegisterId,
  spill_count: StackId,
  spills: ~[Value],
  unhandled: ~[IntervalId],
  active: ~[IntervalId],
  inactive: ~[IntervalId]
}

pub trait Allocator {
  // Prepare for allocation
  pub fn prepare(&mut self);

  // Allocate registers
  pub fn allocate(&mut self, config: Config) -> Result<AllocatorResult, ~str>;
}

enum SplitConf {
  Between(InstrId, InstrId),
  At(InstrId)
}

trait AllocatorHelper {
  // Walk unhandled intervals in the order of increasing starting point
  fn walk_intervals(&mut self,
                    group: GroupId,
                    config: Config) -> Result<GroupResult, ~str>;
  // Try allocating free register
  fn allocate_free_reg<'r>(&'r mut self,
                           current: IntervalId,
                           state: &'r mut AllocatorState) -> bool;
  // Allocate blocked register and spill others, or spill interval itself
  fn allocate_blocked_reg<'r>(&'r mut self,
                              current: IntervalId,
                              state: &'r mut AllocatorState)
      -> Result<(), ~str>;
  // Add movements on block edges
  fn resolve_data_flow(&mut self, list: &[BlockId]);

  // Build live ranges for each interval
  fn build_ranges(&mut self, blocks: &[BlockId], config: Config)
      -> Result<(), ~str>;

  // Split intervals with fixed uses
  fn split_fixed(&mut self);

  //
  // Helpers
  //

  // Sort unhandled list (after insertion)
  fn sort_unhandled<'r>(&'r mut self, state: &'r mut AllocatorState);

  // Get register hint if present
  fn get_hint(&mut self, current: IntervalId) -> Option<RegisterId>;

  // Split interval at some optimal position and add split child to unhandled
  fn split<'r>(&'r mut self,
               current: IntervalId,
               conf: SplitConf,
               state: &'r mut AllocatorState) -> IntervalId;

  // Split and spill all intervals intersecting with current
  fn split_and_spill<'r>(&'r mut self,
                         current: IntervalId,
                         state: &'r mut AllocatorState);

  // Iterate through all active intervals
  fn each_active<'r>(&'r self,
                     state: &'r AllocatorState,
                     f: &fn(i: &IntervalId, reg: RegisterId) -> bool) -> bool;

  // Iterate through all inactive intervals that are intersecting with current
  fn each_intersecting<'r>(&'r self,
                           current: IntervalId,
                           state: &'r AllocatorState,
                           f: &fn(i: &IntervalId,
                                  reg: RegisterId,
                                  pos: InstrId) -> bool) -> bool;

  // Verify allocation results
  fn verify(&self);
}

impl<K: KindHelper+Copy> Allocator for Graph<K> {
  fn prepare(&mut self) {
    if self.prepared {
      return;
    }

    // Get flat list of blocks
    self.flatten();

    // Build live_in/live_out
    self.liveness_analysis();

    self.prepared = true;
  }

  fn allocate(&mut self, config: Config) -> Result<AllocatorResult, ~str> {
    self.prepare();

    // Create physical fixed intervals
    for config.register_groups.eachi() |group, &count| {
      self.physical.insert(group, ~SmallIntMap::new());
      for uint::range(0, count) |reg| {
        let interval = Interval::new(self, 0);
        self.get_interval(&interval).value = RegisterVal(group, reg);
        self.get_interval(&interval).fixed = true;
        self.physical.find_mut(&group).unwrap().insert(reg, interval);
      }
    }

    let list = self.get_block_list();

    // Create live ranges
    match self.build_ranges(list, copy config) {
      Ok(_) => {
        let mut results = ~[];
        // In each register group
        for config.register_groups.eachi() |group, _| {
          // Walk intervals!
          match self.walk_intervals(group, copy config) {
            Ok(res) => {
              results.push(res);
            },
            Err(reason) => { return Err(reason); }
          }
        }

        // Add moves between blocks
        self.resolve_data_flow(list);

        // Resolve parallel moves
        self.resolve_gaps();

        // Verify correctness of allocation
        self.verify();

        // Map results from each group to a general result
        return Ok(AllocatorResult {
          spill_count: do results.map() |result| {
            result.spill_count
          }
        });
      },
      Err(reason) => { return Err(reason); }
    };
  }
}

impl<K: KindHelper+Copy> AllocatorHelper for Graph<K> {
  fn walk_intervals(&mut self,
                    group: GroupId,
                    config: Config) -> Result<GroupResult, ~str> {
    // Initialize allocator state
    let reg_count = config.register_groups[group];
    let mut state = ~AllocatorState {
      config: config,
      group: group,
      register_count: reg_count,
      spill_count: 0,
      spills: ~[],
      unhandled: ~[],
      active: ~[],
      inactive: ~[]
    };

    // We'll work with intervals that contain any ranges
    for self.intervals.each() |_, interval| {
      if interval.value.group() == group && interval.ranges.len() > 0 {
        if interval.fixed {
          // Push all physical registers to active
          state.active.push(interval.id);
        } else {
          // And everything else to unhandled
          state.unhandled.push(interval.id);
        }
      }
    }
    self.sort_unhandled(state);

    while state.unhandled.len() > 0 {
      let current = state.unhandled.shift();
      let position = self.get_interval(&current).start();

      // active => inactive or handled
      let mut handled = ~[];
      do state.active.retain |id| {
        if self.get_interval(id).covers(position) {
          true
        } else {
          if position <= self.get_interval(id).end() {
            state.inactive.push(*id);
          }
          handled.push(self.get_interval(id).value);
          false
        }
      };

      // inactive => active or handled
      do state.inactive.retain |id| {
        if self.get_interval(id).covers(position) {
          state.active.push(*id);
          handled.push(self.get_interval(id).value);
          false
        } else {
          position < self.get_interval(id).end()
        }
      };

      // Return handled spills
      for handled.each() |v| {
        state.to_handled(*v)
      }

      // Skip non-virtual intervals
      if self.intervals.get(&current).value.is_virtual() {
        // Allocate free register
        if !self.allocate_free_reg(current, state) {
          // Or spill some active register
          match self.allocate_blocked_reg(current, state) {
            Ok(_) => (),
            Err(err) => {
              return Err(err);
            }
          }
        }
      }

      // Push register interval to active
      match self.intervals.get(&current).value {
        RegisterVal(_, _) => state.active.push(current),
        _ => ()
      }
    }

    return Ok(GroupResult { spill_count: state.spill_count });
  }

  fn allocate_free_reg<'r>(&'r mut self,
                           current: IntervalId,
                           state: &'r mut AllocatorState) -> bool {
    let mut free_pos = vec::from_elem(state.register_count, uint::max_value);
    let hint = self.get_hint(current);

    // All active intervals use registers
    for self.each_active(state) |_, reg| {
      free_pos[reg] = 0;
    }

    // All inactive registers will eventually use registers
    for self.each_intersecting(current, state) |_, reg, pos| {
      if free_pos[reg] > pos {
        free_pos[reg] = pos;
      }
    }

    // Choose register with maximum free_pos
    let mut reg = 0;
    let mut max_pos = 0;
    match self.intervals.get(&current).next_fixed_use(0) {
      // Intervals with fixed use should have specific register
      Some(u) => {
        match u.kind {
          UseFixed(_, r) => {
            reg = r;
            max_pos = free_pos[reg];
          },
          _ => fail!("Unexpected use kind")
        }
      },

      // Other intervals should prefer register that's free for a longer time
      None => {
        // Prefer hinted register
        match hint {
          Some(hint) => for free_pos.eachi() |i, &pos| {
            if pos > max_pos || hint == i && pos == max_pos {
              max_pos = pos;
              reg = i;
            }
          },
          None => for free_pos.eachi() |i, &pos| {
            if pos > max_pos {
              max_pos = pos;
              reg = i;
            }
          }
        }
      }
    }

    if max_pos == 0 {
      // All registers are blocked - failure
      return false;
    }

    let start = self.intervals.get(&current).start();
    let end = self.intervals.get(&current).end();
    if max_pos >= end {
      // Register is available for whole current's lifetime
    } else if start + 1 >= max_pos {
      // Allocation is impossible
      return false;
    } else {
      // Register is available for some part of current's lifetime
      assert!(max_pos < end);

      let mut split_pos = self.optimal_split_pos(state.group, start, max_pos);
      if split_pos == max_pos - 1 && self.clobbers(state.group, &max_pos) {
        // Splitting right before `call` instruction is pointless,
        // unless we have a register use at that instruction,
        // try spilling current instead.
        match self.intervals.get(&current).next_use(max_pos) {
          Some(u) if u.pos == max_pos => {
            split_pos = max_pos;
          },
          _ => {
            return false;
          }
        }
      }
      let child = self.split(current, At(split_pos), state);

      // Fast case, spill child if there're no register uses after split
      match self.intervals.get(&child).next_use(0) {
        None => {
          self.get_interval(&child).value = state.get_spill();
        },
        _ => ()
      }
    }

    // Give current a register
    self.get_interval(&current).value = RegisterVal(state.group, reg);

    return true;
  }

  fn allocate_blocked_reg<'r>(&'r mut self,
                              current: IntervalId,
                              state: &'r mut AllocatorState)
      -> Result<(), ~str> {
    let mut use_pos = vec::from_elem(state.register_count, uint::max_value);
    let mut block_pos = vec::from_elem(state.register_count, uint::max_value);
    let start = self.get_interval(&current).start();
    let hint = self.get_hint(current);

    // Populate use_pos from every non-fixed interval
    for self.each_active(state) |id, reg| {
      let interval = self.intervals.get(id);
      if !interval.fixed {
        match interval.next_use(start) {
          Some(u) => if use_pos[reg] > u.pos {
            use_pos[reg] = u.pos;
          },
          None => ()
        }
      }
    }
    for self.each_intersecting(current, state) |id, reg, _| {
      let interval = self.intervals.get(id);
      if !interval.fixed {
        match interval.next_use(start) {
          Some(u) => if use_pos[reg] > u.pos {
            use_pos[reg] = u.pos;
          },
          None => ()
        }
      }
    }

    // Populate block_pos from every fixed interval
    for self.each_active(state) |id, reg| {
      if self.intervals.get(id).fixed {
        block_pos[reg] = 0;
        use_pos[reg] = 0;
      }
    }
    for self.each_intersecting(current, state) |id, reg, pos| {
      if self.intervals.get(id).fixed {
        block_pos[reg] = pos;
        if use_pos[reg] > pos {
          use_pos[reg] = pos;
        }
      }
    }

    // Find register with the farest use
    let mut reg = 0;
    let mut max_pos = 0;
    match self.intervals.get(&current).next_fixed_use(0) {
      // Intervals with fixed use should have specific register
      Some(u) => {
        match u.kind {
          UseFixed(_, r) => {
            reg = r;
            max_pos = use_pos[reg];
          },
          _ => fail!("Unexpected use kind")
        }
      },

      // Other intervals should prefer register that isn't used for longer time
      None => {
        // Prefer hinted register
        match hint {
          Some(hint) => for use_pos.eachi() |i, &pos| {
            if pos > max_pos || hint == i && pos == max_pos {
              max_pos = pos;
              reg = i;
            }
          },
          None => for use_pos.eachi() |i, &pos| {
            if pos > max_pos {
              max_pos = pos;
              reg = i;
            }
          }
        }
      }
    }

    let first_use = self.get_interval(&current).next_use(0);
    match first_use {
      Some(u) => {
        if max_pos < u.pos {
          if u.pos == start {
            return Err(~"Incorrect input, allocation impossible");
          }

          // Spill current itself
          self.get_interval(&current).value = state.get_spill();

          // And split before first register use
          self.split(current, Between(start, u.pos), state);
        } else {
          // Assign register to current
          self.get_interval(&current).value = RegisterVal(state.group, reg);

          // If blocked somewhere before end by fixed interval
          if block_pos[reg] <= self.get_interval(&current).end() {
            // Split before this position
            self.split(current, Between(start, block_pos[reg]), state);
          }

          // Split and spill, active and intersecting inactive
          self.split_and_spill(current, state);
        }
      },
      None => {
        // Spill current, it has no uses
        self.get_interval(&current).value = state.get_spill();
      }
    }
    return Ok(());
  }

  fn each_active<'r>(&'r self,
                     state: &'r AllocatorState,
                     f: &fn(i: &IntervalId, reg: RegisterId) -> bool) -> bool {
    for state.active.each() |id| {
      match self.intervals.get(id).value {
        RegisterVal(_, reg) => if !f(id, reg) { break },
        _ => fail!("Expected register in active")
      };
    }
    true
  }

  fn each_intersecting<'r>(&'r self,
                           current: IntervalId,
                           state: &'r AllocatorState,
                           f: &fn(i: &IntervalId,
                                  reg: RegisterId,
                                  pos: InstrId) -> bool) -> bool {
    for state.inactive.each() |id| {
      match self.get_intersection(id, &current) {
        Some(pos) => match self.intervals.get(id).value {
          RegisterVal(_, reg) => if !f(id, reg, pos) { break },
          _ => fail!("Expected register in inactive")
        },
        None => ()
      };
    }
    true
  }

  fn sort_unhandled<'r>(&'r mut self, state: &'r mut AllocatorState) {
    // Sort intervals in the order of increasing start position
    do quick_sort(state.unhandled) |left, right| {
      let lstart = self.get_interval(left).start();
      let rstart = self.get_interval(right).start();

      lstart <= rstart
    };
  }

  fn get_hint(&mut self, current: IntervalId) -> Option<RegisterId> {
    match self.intervals.get(&current).hint {
      Some(ref id) => match self.intervals.get(id).value {
        RegisterVal(g, r) => {
          assert!(g == self.intervals.get(&current).value.group());
          Some(r)
        },
        _ => None
      },
      None => None
    }
  }

  fn split<'r>(&'r mut self,
               current: IntervalId,
               conf: SplitConf,
               state: &'r mut AllocatorState) -> IntervalId {
    let split_pos = match conf {
      Between(start, end) => self.optimal_split_pos(state.group, start, end),
      At(pos) => pos
    };

    let res = self.split_at(&current, split_pos);
    state.unhandled.push(res);
    self.sort_unhandled(state);
    return res;
  }

  fn split_and_spill<'r>(&'r mut self,
                         current: IntervalId,
                         state: &'r mut AllocatorState) {
    let reg = match self.intervals.get(&current).value {
      RegisterVal(_, r) => r,
      _ => fail!("Expected register value")
    };
    let start = self.intervals.get(&current).start();

    // Filter out intersecting intervals
    let mut to_split = ~[];
    for self.each_active(state) |id, _reg| {
      if _reg == reg {
        to_split.push(*id);
      }
    }
    for self.each_intersecting(current, state) |id, _reg, _| {
      if _reg == reg {
        to_split.push(*id);
      }
    }

    // Split and spill!
    for to_split.each() |id| {
      // Spill before or at start of `current`
      let spill_pos = if self.clobbers(state.group, &start) ||
                         self.is_gap(&start) {
        start
      } else {
        start - 1
      };
      let last_use = match self.intervals.get(id).last_use(spill_pos) {
        Some(u) => u.pos,
        None => self.intervals.get(id).start()
      };

      let spill_child = self.split(*id, Between(last_use, spill_pos), state);
      self.get_interval(&spill_child).value = state.get_spill();

      // Split before next register use position
      match self.intervals.get(&spill_child).next_use(spill_pos) {
        Some(u) => {
          self.split(*id, Between(spill_pos, u.pos), state);
        },

        // Let it be spilled for the rest of lifetime
        None() => ()
      }
    };
  }

  fn resolve_data_flow(&mut self, list: &[BlockId]) {
    for list.each() |block_id| {
      let block_end = self.blocks.get(block_id).end() - 1;
      let successors = copy self.blocks.get(block_id).successors;
      for successors.each() |succ_id| {
        let succ_start = copy self.blocks.get(succ_id).start();
        let live_in = copy self.blocks.get(succ_id).live_in;

        for live_in.each() |interval_id| {
          let parent = match self.intervals.get(interval_id).parent {
            Some(p) => p,
            None => *interval_id
          };

          let from = self.child_at(&parent, block_end)
                         .expect("Interval should exist at pred end");
          let to = self.child_at(&parent, succ_start)
                       .expect("Interval should exist at succ start");
          if from != to {
            let gap_pos = if successors.len() == 2 {
              succ_start
            } else {
              block_end
            };
            self.get_gap(&gap_pos).add_move(&from, &to);
          }
        }
      }
    }
  }

  fn build_ranges(&mut self, blocks: &[BlockId], config: Config)
      -> Result<(), ~str> {
    let physical = copy self.physical;
    for blocks.each_reverse() |block_id| {
      let instructions = copy self.blocks.get(block_id).instructions;
      let live_out = copy self.blocks.get(block_id).live_out;
      let block_from = self.blocks.get(block_id).start();
      let block_to = self.blocks.get(block_id).end();

      // Assume that each live_out interval lives for the whole time of block
      // NOTE: we'll shorten it later if definition of this interval appears to
      // be in this block
      for live_out.each() |int_id| {
        self.get_interval(int_id).add_range(block_from, block_to);
      }

      for instructions.each_reverse() |&instr_id| {
        let instr = copy *self.instructions.get(&instr_id);

        // Call instructions should swap out all used registers into stack slots
        for config.register_groups.eachi() |group, &count| {
          if instr.kind.clobbers(group) {
            for uint::range(0, count) |reg| {
              self.get_interval(physical.get(&group).get(&reg))
                  .add_range(instr_id, instr_id + 1);
            }
          }
        }

        // Process output
        match instr.output {
          Some(output) => {
            // Call instructions are defining their value after the call
            let group = self.intervals.get(&output).value.group();
            let pos = if instr.kind.clobbers(group) {
              instr_id + 1
            } else {
              instr_id
            };

            if self.get_interval(&output).ranges.len() != 0  {
              // Shorten range if output outlives block, or is used anywhere
              self.get_interval(&output).first_range().start = pos;
            } else {
              // Add short range otherwise
              self.get_interval(&output).add_range(pos, pos + 1);
            }
            let out_kind = instr.kind.result_kind().unwrap();
            self.get_interval(&output).add_use(out_kind, pos);
          },
          None => ()
        }

        // Process temporary
        for instr.temporary.each() |tmp| {
          let group = self.intervals.get(tmp).value.group();
          if instr.kind.clobbers(group) {
            return Err(~"Call instruction can't have temporary registers");
          }
          self.get_interval(tmp).add_range(instr_id, instr_id + 1);
          self.get_interval(tmp).add_use(UseRegister(group), instr_id);
        }

        // Process inputs
        for instr.inputs.eachi() |i, input_instr| {
          let input = self.get_output(input_instr);
          if !self.intervals.get(&input).covers(instr_id) {
            self.get_interval(&input).add_range(block_from, instr_id);
          }
          let kind = instr.kind.use_kind(i);
          self.get_interval(&input).add_use(kind, instr_id);
        }
      }
    }

    // Now split all intervals with fixed uses
    self.split_fixed();

    return Ok(());
  }

  fn split_fixed(&mut self) {
    let mut list = ~[];
    for self.intervals.each() |_, interval| {
      if interval.uses.any(|u| { u.kind.is_fixed() }) {
        list.push(interval.id);
      }
    }
    for list.each() |id| {
      let cur = *id;

      let uses = do (copy self.intervals.get(id).uses).filtered |u| {
        u.kind.is_fixed()
      };

      let mut i = 0;
      while i < uses.len() - 1 {
        // Split between each pair of uses
        let split_pos = self.optimal_split_pos(uses[i].kind.group(),
                                               uses[i].pos,
                                               uses[i + 1].pos);
        self.split_at(&cur, split_pos);

        i += 1;
      }
    }
  }

  #[cfg(test)]
  fn verify(&self) {
    for self.intervals.each() |_, interval| {
      if interval.ranges.len() > 0 {
        // Every interval should have a non-virtual value
        assert!(!interval.value.is_virtual());

        // Each use should receive the same type of input as it has requested
        for interval.uses.each() |u| {
          // Allocated groups should not differ from specified
          assert!(u.kind.group() == interval.value.group());
          match u.kind {
            // Any use - no restrictions
            UseAny(_) => (),
            UseRegister(_) => match interval.value {
              RegisterVal(_, _) => (), // ok
              _ => fail!("Register expected")
            },
            UseFixed(_, r0) => match interval.value {
              RegisterVal(_, r1) if r0 == r1 => (), // ok
              _ => fail!("Expected fixed register")
            }
          }
        }
      }
    }
  }
  #[cfg(not(test))]
  fn verify(&self) {
    // Production mode, no verification
  }
}

impl AllocatorState {
  fn get_spill(&mut self) -> Value {
    return if self.spills.len() > 0 {
      self.spills.shift()
    } else {
      let slot = self.spill_count;
      self.spill_count += 1;
      StackVal(self.group, slot)
    }
  }

  fn to_handled(&mut self, value: Value) {
    match value {
      StackVal(group, slot) => self.spills.push(StackVal(group, slot)),
      _ => ()
    }
  }
}
