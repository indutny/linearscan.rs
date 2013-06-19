use linearscan::{KindHelper, RegisterHelper, GroupHelper};
use linearscan::graph::{Graph, Value, InstrId, BlockId, Gap,
                        Phi, ToPhi, User, Swap, Move};

pub trait Generator<K, G> {
  fn generate(&self, g: &mut G);
}

pub trait GeneratorFunctions<K, G: GroupHelper, R: RegisterHelper<G> > {
  /// Function prologue (stack initialization, etc)
  fn prelude(&mut self);

  /// Function epilogue.
  /// NOTE: might be invoked multiple times, called at the end of
  /// blocks without successors
  fn epilogue(&mut self);

  /// Swap `left` and `right` value
  fn swap(&mut self, left: &Value<G, R>, right: &Value<G, R>);

  /// Move value from `from` to `to`
  fn move(&mut self, from: &Value<G, R>, to: &Value<G, R>);

  /// Block start notification, might be used to relocate labels
  fn block(&mut self, id: BlockId);

  /// Goto block
  fn goto(&mut self, id: BlockId);

  /// Generate instruction
  fn instr(&mut self,
           kind: &K,
           output: Option<Value<G,R> >,
           inputs: &[Value<G, R>],
           temporary: &[Value<G, R>],
           succ: &[BlockId]);
}

pub trait GeneratorHelper<K, GF> {
  fn generate_gap(&self, g: &mut GF, id: &InstrId);
}

impl<G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R>,
     GF: GeneratorFunctions<K, G, R> > Generator<K, GF>
    for Graph<K, G, R> {
  fn generate(&self, g: &mut GF) {
    g.prelude();

    // Invoke functions in order of increasing instruction id
    for self.instructions.each() |id, instr| {
      // Skip phis
      match instr.kind {
        Phi(_) => loop,
        _ => ()
      };

      // Notify about block start
      let block = self.get_block(&instr.block);
      if *id == block.start().to_uint() {
        g.block(block.id);
      }

      // Call instructions and gaps have GapState
      let is_gap = match instr.kind { Gap => true, _ => false };
      if is_gap || self.gaps.contains_key(id) {
        self.generate_gap(g, &InstrId(*id));
      }

      // Non-gap instructions
      if !is_gap {
        // NOTE: call instruction's output is located right after instruction
        let output = match instr.output {
          Some(ref out) => {
            let group = instr.kind.result_kind().unwrap().group();
            self.get_value(out, if instr.kind.clobbers(&group) {
              instr.id.next()
            } else {
              instr.id
            })
          },
          None => None
        };
        let inputs = do instr.inputs.map() |in| {
          self.get_value(&self.get_output(in), instr.id).expect("input")
        };
        let temporary = do instr.temporary.map() |tmp| {
          self.get_value(tmp, instr.id).expect("temporary")
        };
        match instr.kind {
          Phi(_) => (),
          ToPhi(_) => {
            assert!(inputs.len() == 1);
            let out = output.expect("ToPhi output");
            if out != inputs[0] {
              g.move(&inputs[0], &out);
            }
          },
          Gap => (), // handled separately
          User(ref k) => g.instr(k,
                                 output,
                                 inputs,
                                 temporary,
                                 block.successors)
        }
      }

      // Handle last instruction
      if instr.id == block.end().prev() {
        match block.successors.len() {
          0 => g.epilogue(),
          1 => if block.successors[0].to_uint() != block.id.to_uint() + 1 {
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

impl<G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R>,
     GF: GeneratorFunctions<K, G, R> > GeneratorHelper<K, GF>
    for Graph<K, G, R> {
  fn generate_gap(&self, g: &mut GF, id: &InstrId) {
    match self.gaps.find(&id.to_uint()) {
      Some(state) => for state.actions.each() |action| {
        let from = self.get_interval(&action.from).value.clone();
        let to = self.get_interval(&action.to).value.clone();

        match action.kind {
          Swap => g.swap(&from, &to),
          Move => g.move(&from, &to)
        }
      },
      None => ()
    }
  }
}
