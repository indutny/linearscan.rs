use linearscan::graph::{Graph, KindHelper, Value, InstrId, BlockId, Gap,
                        Phi, ToPhi, User, Swap, Move};

pub trait Generator<K, G> {
  fn generate(&self, g: &mut ~G);
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
  fn generate_gap(&self, g: &mut ~G, id: &InstrId);
}

impl<K: KindHelper+Copy+ToStr, G: GeneratorFunctions<K> > Generator<K, G>
    for Graph<K> {
  fn generate(&self, g: &mut ~G) {
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
      if instr.kind.is_call() || is_gap {
        self.generate_gap(g, id);
      }

      // Non-gap instructions
      if !is_gap {
        let output = match instr.output {
          Some(out) => Some(self.intervals.get(&out).value),
          None => None
        };
        let inputs = instr.inputs.map(|in| self.intervals.get(in).value);
        let temporary = instr.temporary.map(|t| self.intervals.get(t).value);
        match instr.kind {
          Phi => fail!("Phi instruction can't be present in graph"),
          ToPhi => {
            assert!(inputs.len() == 1);
            g.move(output.expect("ToPhi output"), inputs[0]);
          },
          Gap => (), // handled separately
          User(k) => g.instr(&k, output, inputs, temporary, block.successors)
        }
      }

      match block.successors.len() {
        0 => g.epilogue(),
        1 => g.goto(block.successors[0]),
        2 => (), // Should be handled in instruction
        _ => fail!("Too much successors")
      }
    }
  }
}

impl<K: KindHelper+Copy+ToStr, G: GeneratorFunctions<K> > GeneratorHelper<K, G>
    for Graph<K> {
  fn generate_gap(&self, g: &mut ~G, id: &InstrId) {
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
