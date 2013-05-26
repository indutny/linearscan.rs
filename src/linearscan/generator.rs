use linearscan::graph::{Graph, KindHelper, Value, InstrId, BlockId, Gap,
                        Phi, ToPhi, User, Swap, Move};

pub trait Generator<K, G> {
  fn generate(&self, g: &mut G);
}

pub trait GeneratorFunctions<K> {
  /// Function prologue (stack initialization, etc)
  fn prelude(&mut self);

  /// Function epilogue.
  /// NOTE: might be invoked multiple times, called at the end of
  /// blocks without successors
  fn epilogue(&mut self);

  /// Swap `left` and `right` value
  fn swap(&mut self, left: Value, right: Value);

  /// Move value from `from` to `to`
  fn move(&mut self, from: Value, to: Value);

  /// Block start notification, might be used to relocate labels
  fn block(&mut self, id: BlockId);

  /// Goto block
  fn goto(&mut self, id: BlockId);

  /// Generate instruction
  fn instr(&mut self,
           kind: &K,
           output: Option<Value>,
           inputs: &[Value],
           temporary: &[Value],
           succ: &[BlockId]);
}

pub trait GeneratorHelper<K, G> {
  fn generate_gap(&self, g: &mut G, id: &InstrId);
}

impl<K: KindHelper+Copy, G: GeneratorFunctions<K> > Generator<K, G>
    for Graph<K> {
  fn generate(&self, g: &mut G) {
    g.prelude();

    // Invoke functions in order of increasing instruction id
    for self.instructions.each() |id, instr| {
      // Notify about block start
      let block = self.blocks.get(&instr.block);
      if *id == block.start() {
        g.block(block.id);
      }

      // Call instructions and gaps have GapState
      let is_gap = match instr.kind { Gap => true, _ => false };
      if is_gap || self.gaps.contains_key(id) {
        self.generate_gap(g, id);
      }

      // Non-gap instructions
      if !is_gap {
        // NOTE: call instruction's output is located right after instruction
        let output = match instr.output {
          Some(ref out) => {
            let group = instr.kind.result_kind().unwrap().group();
            self.get_value(out, if instr.kind.clobbers(group) {
              instr.id + 1
            } else {
              instr.id
            })
          },
          None => None
        };
        let inputs = do instr.inputs.map() |in| {
          self.get_value(in, instr.id).expect("input")
        };
        let temporary = do instr.temporary.map() |tmp| {
          self.get_value(tmp, instr.id).expect("temporary")
        };
        match instr.kind {
          Phi(_) => fail!("Phi instruction can't be present in graph"),
          ToPhi(_) => {
            assert!(inputs.len() == 1);
            let out = output.expect("ToPhi output");
            if out != inputs[0] {
              g.move(inputs[0], out);
            }
          },
          Gap => (), // handled separately
          User(k) => g.instr(&k, output, inputs, temporary, block.successors)
        }
      }

      // Handle last instruction
      if instr.id == block.end() - 1 {
        match block.successors.len() {
          0 => g.epilogue(),
          1 => if block.successors[0] != block.id + 1 {
            // Goto to non-consequent successor
            g.goto(block.successors[0])
          },
          2 => (), // Should be handled in instruction
          _ => fail!("Too much successors")
        }
      }
    }
  }
}

impl<K: KindHelper+Copy, G: GeneratorFunctions<K> > GeneratorHelper<K, G>
    for Graph<K> {
  fn generate_gap(&self, g: &mut G, id: &InstrId) {
    let state = self.gaps.find(id).expect("Gap at instruction");

    for state.actions.each() |action| {
      let from = self.intervals.get(&action.from).value;
      let to = self.intervals.get(&action.to).value;

      match action.kind {
        Swap => g.swap(from, to),
        Move => g.move(from, to)
      }
    }
  }
}
