use linearscan::graph::{Graph, KindHelper, Interval,
                        IntervalId, InstrId, RegisterId,
                        UseFixed,
                        Value, Register, Stack};
use linearscan::flatten::Flatten;
use linearscan::liveness::Liveness;
use std::sort::quick_sort;

pub struct Config {
  register_count: RegisterId
}

struct AllocatorState {
  config: Config,
  spill_count: uint,
  spills: ~[Value],
  unhandled: ~[IntervalId],
  active: ~[IntervalId],
  inactive: ~[IntervalId]
}

pub trait Allocator {
  // Allocate registers
  pub fn allocate(&mut self, config: Config);
}

trait AllocatorHelper {
  fn walk_intervals(&mut self, config: Config);
  fn each_active<'r>(&'r self,
                     state: &'r AllocatorState,
                     f: &fn(i: &IntervalId, reg: RegisterId) -> bool) -> bool;
  fn each_intersecting<'r>(&'r self,
                           current: IntervalId,
                           state: &'r AllocatorState,
                           f: &fn(i: &IntervalId,
                                  reg: RegisterId,
                                  pos: InstrId) -> bool) -> bool;
  fn allocate_free_reg<'r>(&'r mut self,
                           current: IntervalId,
                           state: &'r mut AllocatorState) -> bool;
  fn allocate_blocked_reg<'r>(&'r mut self,
                              current: IntervalId,
                              state: &'r mut AllocatorState);
  fn sort_unhandled<'r>(&'r mut self, state: &'r mut AllocatorState);
  fn split_between<'r>(&'r mut self,
                       current: IntervalId,
                       start: InstrId,
                       end: InstrId,
                       state: &'r mut AllocatorState) -> IntervalId;
  fn split_and_spill<'r>(&'r mut self,
                         current: IntervalId,
                         state: &'r mut AllocatorState);
}

impl<K: KindHelper+Copy+ToStr> Allocator for Graph<K> {
  pub fn allocate(&mut self, config: Config) {
    // Create physical fixed intervals
    for uint::range(0, config.register_count) |i| {
      let interval = Interval::new(self);
      self.get_interval(&interval).value = Register(i);
      self.get_interval(&interval).fixed = true;
      self.physical.push(interval);
    }

    // Get flat list of blocks
    let list = self.flatten();

    // Build live_in/live_out
    self.build_liveranges(list);

    self.walk_intervals(config);
  }
}

impl<K: KindHelper+Copy+ToStr> AllocatorHelper for Graph<K> {
  fn walk_intervals(&mut self, config: Config) {
    let mut state = ~AllocatorState {
      config: config,
      spill_count: 0,
      spills: ~[],
      unhandled: ~[],
      active: ~[],
      inactive: ~[]
    };

    // We'll work with intervals that contain any ranges
    for self.intervals.each() |_, interval| {
      if interval.ranges.len() > 0 {
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

      // Allocate free register
      if !self.allocate_free_reg(current, state) {
        // Or spill some active register
        self.allocate_blocked_reg(current, state);
      }

      // Push register interval to active
      match self.get_interval(&current).value {
        Register(_) => state.active.push(current),
        _ => ()
      }
    }
  }

  fn each_active<'r>(&'r self,
                     state: &'r AllocatorState,
                     f: &fn(i: &IntervalId, reg: RegisterId) -> bool) -> bool {
    for state.active.each() |id| {
      match self.intervals.get(id).value {
        Register(reg) => if !f(id, reg) { break },
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
      match self.get_next_intersection(id, &current) {
        Some(pos) => match self.intervals.get(id).value {
          Register(reg) => if !f(id, reg, pos) { break },
          _ => fail!("Expected register in inactive")
        },
        None => ()
      };
    }
    true
  }

  fn allocate_free_reg<'r>(&'r mut self,
                           current: IntervalId,
                           state: &'r mut AllocatorState) -> bool {
    let mut free_pos = vec::from_elem(state.config.register_count,
                                      uint::max_value);

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
          UseFixed(r) => {
            reg = r;
            max_pos = free_pos[reg];
          },
          _ => fail!("Unexpected use kind")
        }
      },

      // Other intervals should prefer register that's free for a longer time
      None => {
        for free_pos.eachi() |i, pos| {
          if *pos > max_pos {
            max_pos = *pos;
            reg = i;
          }
        }
      }
    }

    if max_pos == 0 {
      // All registers are blocked - failure
      return false;
    }

    self.get_interval(&current).value = Register(reg);
    if max_pos > self.get_interval(&current).end() {
      // Register is available for whole current's lifetime
    } else {
      // Register is available for some part of current's lifetime
      assert!(max_pos < self.get_interval(&current).end());
      let start = self.get_interval(&current).start();
      self.split_between(current, start, max_pos, state);
    }

    return true;
  }

  fn allocate_blocked_reg<'r>(&'r mut self,
                              current: IntervalId,
                              state: &'r mut AllocatorState) {
    let mut use_pos = vec::from_elem(state.config.register_count,
                                     uint::max_value);
    let mut block_pos = vec::from_elem(state.config.register_count,
                                       uint::max_value);
    let start = self.get_interval(&current).start();

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
          UseFixed(r) => {
            reg = r;
            max_pos = use_pos[reg];
          },
          _ => fail!("Unexpected use kind")
        }
      },

      // Other intervals should prefer register that isn't used for longer time
      None => {
        for use_pos.eachi() |i, pos| {
          if *pos > max_pos {
            max_pos = *pos;
            reg = i;
          }
        }
      }
    }

    let first_use = self.get_interval(&current).next_use(0);
    match first_use {
      Some(u) => {
        if max_pos < u.pos {
          // Spill current itself
          self.get_interval(&current).value = state.get_spill();

          // And split before first register use
          self.split_between(current, start, u.pos, state);
        } else {
          // Assign register to current
          self.get_interval(&current).value = Register(reg);

          // If blocked somewhere before end by fixed interval
          if block_pos[reg] <= self.get_interval(&current).end() {
            // Split before this position
            self.split_between(current, start, block_pos[reg], state);
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
  }

  fn sort_unhandled<'r>(&'r mut self, state: &'r mut AllocatorState) {
    // Sort intervals in the order of increasing start position
    do quick_sort(state.unhandled) |left, right| {
      let lstart = self.get_interval(left).start();
      let rstart = self.get_interval(right).start();

      lstart <= rstart
    };
  }

  fn split_between<'r>(&'r mut self,
                       current: IntervalId,
                       start: InstrId,
                       end: InstrId,
                       state: &'r mut AllocatorState) -> IntervalId {
    // TODO(indutny) split at block edges if possible
    let res = self.split_at(&current, end);
    state.unhandled.push(res);
    self.sort_unhandled(state);
    return res;
  }

  fn split_and_spill<'r>(&'r mut self,
                         current: IntervalId,
                         state: &'r mut AllocatorState) {
    let reg = match self.intervals.get(&current).value {
      Register(r) => r,
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
      // Spill after start of `current`
      let spill_child = self.split_at(id, start);
      self.get_interval(&spill_child).value = state.get_spill();

      // Split before next register use position
      match self.intervals.get(id).next_use(start) {
        Some(u) => {
          self.split_between(*id, start, u.pos, state);
        },

        // Let it be spilled for the rest of lifetime
        None() => ()
      }
    };
  }
}

impl AllocatorState {
  fn get_spill(&mut self) -> Value {
    return if self.spills.len() > 0 {
      self.spills.shift()
    } else {
      let slot = self.spill_count;
      self.spill_count += 1;
      Stack(slot)
    }
  }

  fn to_handled(&mut self, value: Value) {
    match value {
      Stack(slot) => self.spills.push(Stack(slot)),
      _ => ()
    }
  }
}
