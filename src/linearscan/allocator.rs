use linearscan::graph::{Graph, KindHelper, Interval, IntervalId};
use linearscan::flatten::Flatten;
use linearscan::liveness::Liveness;
use std::sort::quick_sort;

pub struct Config {
  register_count: uint
}

pub trait Allocator {
  // Allocate registers
  pub fn allocate(&mut self, config: Config);
}

trait AllocatorHelper {
  pub fn walk_intervals(&mut self);
}

impl<K: KindHelper+Copy+ToStr> Allocator for Graph<K> {
  pub fn allocate(&mut self, config: Config) {
    // Create physical fixed intervals
    for uint::range(0, config.register_count) |_| {
      let interval = Interval::new(self);
      self.physical.push(interval);
    }

    // Get flat list of blocks
    let list = self.flatten();

    // Build live_in/live_out
    self.build_liveranges(list);

    self.walk_intervals();
  }
}

impl<K: KindHelper+Copy+ToStr> AllocatorHelper for Graph<K> {
  pub fn walk_intervals(&mut self) {
    let mut unhandled: ~[IntervalId] = ~[];
    let mut active: ~[IntervalId] = ~[];
    let mut inactive: ~[IntervalId] = ~[];

    // We'll work with intervals that contain any ranges
    for self.intervals.each() |_, interval| {
      if interval.ranges.len() > 0 {
        unhandled.push(interval.id);
      }
    };

    // Sort intervals in the order of increasing start position
    do quick_sort(unhandled) |left, right| {
      let lstart = self.get_interval(left).start();
      let rstart = self.get_interval(right).start();

      lstart <= rstart
    };

    while unhandled.len() > 0 {
      let current = unhandled.shift();
      let position = self.get_interval(&current).start();

      // active => inactive or handled
      do active.retain |id| {
        if self.get_interval(id).covers(position) {
          true
        } else {
          if self.get_interval(id).end() > position {
            inactive.push(*id);
          }
          false
        }
      };
    }
  }
}
