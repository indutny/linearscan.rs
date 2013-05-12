use linearscan::graph::{Graph, BlockId};
use std::bitv::BitvSet;

pub trait Liveness {
  fn build_liveranges(&mut self, blocks: &[BlockId]);
}

trait LivenessHelper {
  // Build live_gen, live_kill
  fn build_local(&mut self, blocks: &[BlockId]);

  // Build live_in, live_out
  fn build_global(&mut self, blocks: &[BlockId]);

  // Create ranges for every interval
  fn build_ranges(&mut self, blocks: &[BlockId]);
}

impl<K> Liveness for Graph<K> {
  fn build_liveranges(&mut self, blocks: &[BlockId]) {
    self.build_local(blocks);
    self.build_global(blocks);
    self.build_ranges(blocks);
  }
}

impl<K> LivenessHelper for Graph<K> {
  fn build_local(&mut self, blocks: &[BlockId]) {
    for blocks.each() |block| {
      let instructions = copy self.get_block(block).instructions;

      for instructions.each() |instr| {
        let output = self.get_instr(instr).output;
        let inputs = copy self.get_instr(instr).inputs;

        self.get_block(block).live_kill.insert(output);
        for inputs.each() |input| {
          if !self.get_block(block).live_kill.contains(input) {
            self.get_block(block).live_gen.insert(*input);
          }
        }
      }
    };
  }

  fn build_global(&mut self, blocks: &[BlockId]) {
    let mut change = true;
    while change {
      change = false;

      for blocks.each() |block| {
        let successors = copy self.get_block(block).successors;

        let mut tmp = ~BitvSet::new();
        for successors.each() |succ| {
          tmp.union_with(self.get_block(succ).live_in);
        }

        // Propagate succ.live_in to block.live_out
        if !self.get_block(block).live_out.is_superset(tmp) {
          self.get_block(block).live_out.union_with(tmp);
          change = true;
        }

        // Propagate union(diff(block.live_out, block.live_kill), block.live_gen) to block.live_in
        let mut old = copy self.get_block(block).live_in;
        old.difference_with(self.get_block(block).live_kill);
        old.union_with(self.get_block(block).live_gen);
        if old != self.get_block(block).live_in {
          self.get_block(block).live_in = old;
          change = true;
        }
      }
    }
  }

  fn build_ranges(&mut self, blocks: &[BlockId]) {
    for blocks.each() |id| {
      let instructions = copy self.get_block(id).instructions;

      for instructions.each() |instr| {
        let output = self.get_instr(instr).output;
        let inputs = copy self.get_instr(instr).inputs;

        // If output outlives the block
        if self.get_block(id).live_out.contains(&output) {
        }
      }
    }
  }
}
