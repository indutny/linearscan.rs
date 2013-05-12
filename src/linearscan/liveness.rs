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
      let block_start = self.get_instr(*instructions.head()).flat_id;
      let block_end = self.get_instr(*instructions.last()).flat_id;

      for instructions.each() |instr| {
        let flat_id = self.get_instr(*instr).flat_id;
        let output = self.get_instr(*instr).output;
        let inputs = copy self.get_instr(*instr).inputs;

        self.get_block(*block).live_gen.insert(output);
        self.get_interval(output).add_range(flat_id, block_end);

        for inputs.each() |input| {
          // Interval is live for the part of the block
          if self.get_block(*block).live_gen.contains(input) {
            self.get_interval(*input).extend_range(flat_id);
          } else {
            // Interval is live from the start of the block
            self.get_interval(*input).add_range(block_start, flat_id);
          }
          self.get_block(*block).live_kill.insert(*input);
        };
      };
    };
  }

  fn build_global(&mut self, blocks: &[BlockId]) {
  }
}
