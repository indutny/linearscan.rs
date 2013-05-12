use linearscan::graph::{Graph, BlockId, InstrId, Interval, LiveRange};

pub trait Liveness {
  fn build_liveranges(&mut self, blocks: &[BlockId]);
}

trait LivenessHelper {
  // Build live_gen, live_kill
  fn build_local(&mut self, blocks: &[BlockId]);

  // Build live_in, live_out
  fn build_global(&mut self, blocks: &[BlockId]);
}

impl<K> Liveness for Graph<K> {
  fn build_liveranges(&mut self, blocks: &[BlockId]) {
    self.build_local(blocks);
    self.build_global(blocks);
  }
}

impl<K> LivenessHelper for Graph<K> {
  fn build_local(&mut self, blocks: &[BlockId]) {
    for blocks.each() |block| {
      let instructions = copy self.get_block(*block).instructions;

      for instructions.each() |instr| {
        let output = self.get_instr(*instr).output;
        let inputs = copy self.get_instr(*instr).inputs;

        let block = self.get_block(*block);

        block.live_gen.insert(output);
        for inputs.each() |input| {
          block.live_kill.insert(*input);
        };
      };
    };
  }

  fn build_global(&mut self, blocks: &[BlockId]) {
  }
}
