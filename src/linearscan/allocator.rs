use linearscan::graph::{Graph, KindHelper, Interval};
use linearscan::flatten::Flatten;
use linearscan::liveness::Liveness;

pub struct Config {
  register_count: uint
}

pub trait Allocator {
  // Allocate registers
  pub fn allocate(&mut self, config: Config);
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
  }
}
