use linearscan::graph::{GraphBuilder, BlockId};
use linearscan::flatten::Flatten;

pub struct Config {
  register_count: uint
}

pub trait Allocator {
  // Allocate registers
  pub fn allocate(&mut self, config: Config);
}

impl<K> Allocator for GraphBuilder<K> {

  pub fn allocate(&mut self, config: Config) {
    let list = self.flatten();
    io::println(fmt!("%?", list));
  }
}
