use linearscan::graph::{Graph, KindHelper, Interval,
                        IntervalId, InstrId, Register};
use linearscan::flatten::Flatten;
use linearscan::liveness::Liveness;
use std::sort::quick_sort;

pub struct Config {
  register_count: uint
}

struct AllocatorState {
  config: Config,
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
  fn allocate_free_reg<'r>(&'r mut self,
                           current: IntervalId,
                           state: &'r mut AllocatorState) -> bool;
  fn allocate_blocked_reg<'r>(&'r mut self,
                              current: IntervalId,
                              state: &'r mut AllocatorState);
  fn sort_unhandled<'r>(&'r mut self, state: &'r mut AllocatorState);
  fn split_before<'r>(&'r mut self,
                      current: IntervalId,
                      pos: InstrId,
                      state: &'r mut AllocatorState) -> IntervalId;
}

impl<K: KindHelper+Copy+ToStr> Allocator for Graph<K> {
  pub fn allocate(&mut self, config: Config) {
    // Create physical fixed intervals
    for uint::range(0, config.register_count) |i| {
      let interval = Interval::new(self);
      self.get_interval(&interval).value = Register(i);
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
      unhandled: ~[],
      active: ~[],
      inactive: ~[]
    };

    // We'll work with intervals that contain any ranges
    for self.intervals.each() |_, interval| {
      if interval.ranges.len() > 0 {
        match interval.value {
          // Push all physical registers to active
          Register(_) => state.active.push(interval.id),

          // And everything else to unhandled
          _ => state.unhandled.push(interval.id)
        }
      }
    }
    self.sort_unhandled(state);

    while state.unhandled.len() > 0 {
      let current = state.unhandled.shift();
      let position = self.get_interval(&current).start();

      // active => inactive or handled
      do state.active.retain |id| {
        if self.get_interval(id).covers(position) {
          true
        } else {
          if position <= self.get_interval(id).end() {
            state.inactive.push(*id);
          }
          false
        }
      };

      // inactive => active or handled
      do state.inactive.retain |id| {
        if self.get_interval(id).covers(position) {
          state.active.push(*id);
          false
        } else {
          position < self.get_interval(id).end()
        }
      };

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

  fn allocate_free_reg<'r>(&'r mut self,
                           current: IntervalId,
                           state: &'r mut AllocatorState) -> bool {
    let mut free_pos = vec::from_elem(state.config.register_count,
                                      uint::max_value);

    // All active intervals use registers
    for state.active.each() |id| {
      match self.get_interval(id).value {
        Register(reg) => free_pos[reg] = 0,
        _ => fail!("Active interval should have register value")
      }
    }

    // All inactive registers will eventually use registers
    for state.inactive.each() |id| {
      match self.get_next_intersection(id, &current) {
        Some(pos) => match self.get_interval(id).value {
          Register(reg) => free_pos[reg] = pos,
          _ => fail!("Active interval should have register value")
        },
        None => ()
      }
    }

    // Choose register with maximum free_pos
    let mut reg = 0;
    let mut max_pos = 0;
    for free_pos.eachi() |i, pos| {
      if *pos > max_pos {
        max_pos = *pos;
        reg = i;
      }
    };

    if max_pos == 0 {
      // All registers are blocked - faiulre
      return false;
    }

    self.get_interval(&current).value = Register(reg);
    if max_pos > self.get_interval(&current).end() {
      // Register is available for whole current's lifetime
    } else {
      // Register is available for some part of current's lifetime
      assert!(max_pos < self.get_interval(&current).end());
      self.split_before(current, max_pos, state);
    }

    return true;
  }

  fn allocate_blocked_reg<'r>(&'r mut self,
                              current: IntervalId,
                              state: &'r mut AllocatorState) {
  }

  fn sort_unhandled<'r>(&'r mut self, state: &'r mut AllocatorState) {
    // Sort intervals in the order of increasing start position
    do quick_sort(state.unhandled) |left, right| {
      let lstart = self.get_interval(left).start();
      let rstart = self.get_interval(right).start();

      lstart <= rstart
    };
  }

  fn split_before<'r>(&'r mut self,
                      current: IntervalId,
                      pos: InstrId,
                      state: &'r mut AllocatorState) -> IntervalId {
    // TODO(indutny) split at block edges if possible
    let res = self.split_at(&current, pos);
    state.unhandled.push(res);
    self.sort_unhandled(state);
    return res;
  }
}
