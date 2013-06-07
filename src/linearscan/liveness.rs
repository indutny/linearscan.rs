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
    let blocks = self.get_block_list();
    self.build_local(blocks);
    self.build_global(blocks);
  }
}

impl<K: KindHelper+Copy> LivenessHelper for Graph<K> {
  fn build_local(&mut self, blocks: &[BlockId]) {
    for blocks.each() |block| {
      let instructions = copy self.get_block(block).instructions;

      for instructions.each() |instr| {
        let output = self.get_instr(instr).output;
        let inputs = copy self.get_instr(instr).inputs;

        match output {
          Some(output) => self.get_mut_block(block).live_kill
                              .insert(output.to_uint()),
          None => true
        };

        for inputs.each() |input_instr| {
          let input = self.get_output(input_instr);
          if !self.get_block(block).live_kill.contains(&input.to_uint()) {
            self.get_mut_block(block).live_gen.insert(input.to_uint());
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
        let successors = copy self.get_block(block).successors;

        let mut tmp = ~BitvSet::new();
        for successors.each() |succ| {
          tmp.union_with(self.get_block(succ).live_in);
        }

        // Propagate succ.live_in to block.live_out
        if self.get_block(block).live_out != tmp {
          self.get_mut_block(block).live_out = tmp;
          change = true;
        }

        // Propagate:
        // `union(diff(block.live_out, block.live_kill), block.live_gen)`
        // to block.live_in
        let mut old = copy self.get_block(block).live_out;
        old.difference_with(self.get_block(block).live_kill);
        old.union_with(self.get_block(block).live_gen);
        if old != self.get_block(block).live_in {
          self.get_mut_block(block).live_in = old;
          change = true;
        }
      }
    }
  }

}
