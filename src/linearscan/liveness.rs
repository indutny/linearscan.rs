use linearscan::graph::{Graph, BlockId, KindHelper};
use extra::bitv::BitvSet;

pub trait Liveness {
  fn liveness_analysis(&mut self);
}

trait LivenessHelper {
  // Build live_gen, live_kill
  fn build_local(&mut self, blocks: &[BlockId]);

  // Build live_in, live_out
  fn build_global(&mut self, blocks: &[BlockId]);
}

impl<K: KindHelper+Copy> Liveness for Graph<K> {
  fn liveness_analysis(&mut self) {
    let mut blocks = ~[];
    for self.blocks.each() |_, block| {
      blocks.push(block.id);
    }
    self.build_local(blocks);
    self.build_global(blocks);
  }
}

impl<K: KindHelper+Copy> LivenessHelper for Graph<K> {
  fn build_local(&mut self, blocks: &[BlockId]) {
    for blocks.each() |block| {
      let instructions = copy self.blocks.get(block).instructions;

      for instructions.each() |instr| {
        let output = self.instructions.get(instr).output;
        let inputs = copy self.instructions.get(instr).inputs;

        match output {
          Some(output) => self.get_block(block).live_kill.insert(output),
          None => true
        };

        for inputs.each() |&input| {
          if !self.blocks.get(block).live_kill.contains(&input) {
            self.get_block(block).live_gen.insert(input);
          }
        }
      }
    };
  }

  fn build_global(&mut self, blocks: &[BlockId]) {
    let mut change = true;
    while change {
      change = false;

      for blocks.each_reverse() |block| {
        let successors = copy self.blocks.get(block).successors;

        let mut tmp = ~BitvSet::new();
        for successors.each() |succ| {
          tmp.union_with(self.blocks.get(succ).live_in);
        }

        // Propagate succ.live_in to block.live_out
        if self.blocks.get(block).live_out != tmp {
          self.get_block(block).live_out = tmp;
          change = true;
        }

        // Propagate:
        // `union(diff(block.live_out, block.live_kill), block.live_gen)`
        // to block.live_in
        let mut old = copy self.blocks.get(block).live_out;
        old.difference_with(self.blocks.get(block).live_kill);
        old.union_with(self.blocks.get(block).live_gen);
        if old != self.blocks.get(block).live_in {
          self.get_block(block).live_in = old;
          change = true;
        }
      }
    }
  }

}
