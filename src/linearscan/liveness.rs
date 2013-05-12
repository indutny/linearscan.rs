use linearscan::graph::{Graph, BlockId, InstrId, Interval, LiveRange};

pub trait Liveness {
  fn build_liveranges(&mut self);
}

trait LivenessHelper {
  // Build live_gen, live_kill
  fn build_local(&mut self);

  // Build live_in, live_out
  fn build_global(&mut self);
}

impl<K> Liveness for Graph<K> {
  fn build_liveranges(&mut self) {
    self.build_local();
    self.build_global();
  }
}

impl<K> LivenessHelper for Graph<K> {
  fn build_local(&mut self) {
  }

  fn build_global(&mut self) {
  }
}
