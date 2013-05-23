use linearscan::graph::{Graph, BlockId, KindHelper, Instruction, UseRegister};
use extra::bitv::BitvSet;

pub trait Liveness {
  fn build_liveranges(&mut self, blocks: &[BlockId]) -> Result<(), ~str>;
}

trait LivenessHelper {
  // Build live_gen, live_kill
  fn build_local(&mut self, blocks: &[BlockId]);

  // Build live_in, live_out
  fn build_global(&mut self, blocks: &[BlockId]);

  // Build live ranges
  fn build_ranges(&mut self, blocks: &[BlockId]) -> Result<(), ~str>;

  // Split intervals with fixed uses
  fn split_fixed(&mut self);
}

impl<K: KindHelper+Copy+ToStr> Liveness for Graph<K> {
  fn build_liveranges(&mut self, blocks: &[BlockId]) -> Result<(), ~str> {
    self.build_local(blocks);
    self.build_global(blocks);
    return do self.build_ranges(blocks).chain() |_| {
      self.split_fixed();
      Ok(())
    };
  }
}

impl<K: KindHelper+Copy+ToStr> LivenessHelper for Graph<K> {
  fn build_local(&mut self, blocks: &[BlockId]) {
    for blocks.each() |block| {
      let instructions = copy self.get_block(block).instructions;

      for instructions.each() |instr| {
        let output = self.get_instr(instr).output;
        let inputs = copy self.get_instr(instr).inputs;

        match output {
          Some(output) => self.get_block(block).live_kill.insert(output),
          None => true
        };

        for inputs.each() |&input| {
          if !self.get_block(block).live_kill.contains(&input) {
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
        let successors = copy self.get_block(block).successors;

        let mut tmp = ~BitvSet::new();
        for successors.each() |succ| {
          tmp.union_with(self.get_block(succ).live_in);
        }

        // Propagate succ.live_in to block.live_out
        if self.get_block(block).live_out != tmp {
          self.get_block(block).live_out = tmp;
          change = true;
        }

        // Propagate:
        // `union(diff(block.live_out, block.live_kill), block.live_gen)`
        // to block.live_in
        let mut old = copy self.get_block(block).live_out;
        old.difference_with(self.get_block(block).live_kill);
        old.union_with(self.get_block(block).live_gen);
        if old != self.get_block(block).live_in {
          self.get_block(block).live_in = old;
          change = true;
        }
      }
    }
  }

  fn build_ranges(&mut self, blocks: &[BlockId]) -> Result<(), ~str> {
    let physical = copy self.physical;
    for blocks.each_reverse() |block_id| {
      let instructions = copy self.get_block(block_id).instructions;
      let live_out = copy self.get_block(block_id).live_out;
      let block_from = *instructions.head();
      let block_to = instructions.last() + 1;

      // Assume that each live_out interval lives for the whole time of block
      // NOTE: we'll shorten it later if definition of this interval appears to
      // be in this block
      for live_out.each() |int_id| {
        self.get_interval(int_id).add_range(block_from, block_to);
      }

      for instructions.each_reverse() |&instr_id| {
        let instr: ~Instruction<K> = copy *self.instructions.get(&instr_id);

        // Call instructions should swap out all used registers into stack slots
        if instr.kind.is_call() {
          for physical.each() |reg| {
            self.get_interval(reg).add_range(instr_id, instr_id + 1);
          }
        }

        // Process output
        match instr.output {
          Some(output) => {
            // Call instructions are defining their value after the call
            let pos = if instr.kind.is_call() {
              instr_id + 1
            } else {
              instr_id
            };

            if self.get_interval(&output).ranges.len() != 0  {
              // Shorten range if output outlives block, or is used anywhere
              self.get_interval(&output).first_range().start = pos;
            } else {
              // Add short range otherwise
              self.get_interval(&output).add_range(pos, pos + 1);
            }
            let out_kind = instr.kind.result_kind().unwrap();
            self.get_interval(&output).add_use(out_kind, pos);
          },
          None => ()
        }

        // Process temporary
        if instr.kind.is_call() {
          if instr.temporary.len() != 0 {
            return Err(~"Call instruction can't have temporary registers");
          }
        } else {
          for instr.temporary.each() |tmp| {
            self.get_interval(tmp).add_range(instr_id, instr_id + 1);
            self.get_interval(tmp).add_use(UseRegister, instr_id);
          }
        }

        // Process inputs
        for instr.inputs.eachi() |i, input| {
          if !self.intervals.get(input).covers(instr_id) {
            self.get_interval(input).add_range(block_from, instr_id);
          }
          let kind = instr.kind.use_kind(i);
          self.get_interval(input).add_use(kind, instr_id);
        }
      }
    }

    return Ok(());
  }

  fn split_fixed(&mut self) {
    let mut list = ~[];
    for self.intervals.each() |_, interval| {
      if interval.uses.any(|u| { u.kind.is_fixed() }) {
        list.push(interval.id);
      }
    }
    for list.each() |id| {
      let start = self.intervals.get(id).start();
      let end = self.intervals.get(id).end();

      let uses = do (copy self.intervals.get(id).uses).filtered |u| {
        u.kind.is_fixed()
      };

      for uses.each() |u| {
        let pos = if u.pos == start {
          if start + 1 < end {
            Some(u.pos + 1)
          } else {
            None
          }
        } else if u.pos == end {
          if end > start + 1 {
            Some(u.pos - 1)
          } else {
            None
          }
        } else {
          Some(u.pos)
        };

        match pos {
          Some(pos) if self.intervals.get(id).covers(pos) => {
            self.split_at(id, pos);
          },
          _ => ()
        }
      };
    }
  }
}
