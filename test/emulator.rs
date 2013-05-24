use linearscan::{Graph};
use linearscan::graph::{User, Gap, Phi, ToPhi, Value, Register, Stack, InstrId};
use extra::smallintmap::SmallIntMap;

#[deriving(Eq, ToStr)]
pub enum Kind {
  Increment,
  BranchIfBigger,
  AB,
  JustUse,
  Print,
  Zero,
  Ten,
  Return
}

pub struct Emulator {
  ip: InstrId,
  result: Option<uint>,
  registers: ~SmallIntMap<uint>,
  stack: ~SmallIntMap<uint>
}

enum MoveStatus {
  Moving,
  Moved
}

pub impl Emulator {
  fn new() -> Emulator {
    Emulator {
      ip: 0,
      result: None,
      registers: ~SmallIntMap::new(),
      stack: ~SmallIntMap::new()
    }
  }

  fn run(&mut self, graph: &Graph<Kind>) -> uint {
    loop {
      // Execution finished
      if self.result.is_some() {
        return self.result.unwrap();
      }

      let instr = graph.instructions.find(&self.ip).expect("No OOB");

      // Get output, temporaries
      let output = match instr.output {
        Some(out) => Some(graph.get_value(&out, self.ip).unwrap()),
        None => None
      };
      let tmps = do instr.temporary.map() |tmp| {
        graph.get_value(tmp, self.ip).unwrap()
      };
      // And inputs
      let inputs = do instr.inputs.map() |input| {
        self.read(graph.get_value(input, self.ip).expect("input"))
      };

      // Get successor positions
      let succ = do graph.blocks.get(&instr.block).successors.map() |succ| {
        graph.blocks.get(succ).start()
      };

      match instr.kind {
        Phi => fail!("Impossible, phi should not be executed"),
        ToPhi => { self.put(output, inputs[0]); self.ip += 1; },
        Gap => self.parallel_move(graph),
        User(usr) => self.user_instruction(graph,
                                           usr,
                                           output,
                                           tmps,
                                           inputs,
                                           succ)
      };

      // Goto
      if self.ip > graph.blocks.get(&instr.block).end() {
        assert!(succ.len() == 1);
        self.ip = succ[0];
      }
    }
  }

  fn read(&self, slot: Value) -> uint {
    match slot {
      Register(r) => *self.registers.get(&r),
      Stack(s) => *self.stack.get(&s),
      _ => fail!()
    }
  }

  fn put(&mut self, slot: Option<Value>, value: uint) {
    match slot.expect("Write to slot") {
      Register(r) => { self.registers.insert(r, value); },
      Stack(s) => { self.stack.insert(s, value); },
      _ => fail!()
    }
  }

  fn parallel_move(&mut self, graph: &Graph<Kind>) {
    let gap = graph.gaps.get(&self.ip);

    // Find loops
    let mut loop_i = None;
    for gap.moves.eachi() |i, move| {
      let from = graph.intervals.get(&move.from).value;
      let to = graph.intervals.get(&move.to).value;

      if from != to {
        let mut j = i + 1;
        while j < gap.moves.len() {
          let next_to = graph.intervals.get(&gap.moves[j].from).value;
          if next_to == from {
            loop_i = Some((i, j));
            break;
          }
          j += 1;
        }
      }

      if loop_i.is_some() {
        break;
      }
    }

    // Process loop end
    let tmp = match loop_i {
      Some((_, end)) => {
        let to = graph.intervals.get(&gap.moves[end].to).value;
        Some(self.read(to))
      },
      _ => None
    };

    // Process other moves
    for gap.moves.eachi() |i, move| {
      let from = graph.intervals.get(&move.from).value;
      let to = graph.intervals.get(&move.to).value;

      match loop_i {
        Some((a, _)) if i == a => {
          // Loop start
          self.put(Some(to), tmp.expect("Tmp should be present"));
        },
        _ => {
          // Normal
          let val = self.read(from);
          self.put(Some(to), val);
        }
      }
    }

    self.ip += 1;
  }

  fn user_instruction(&mut self,
                      graph: &Graph<Kind>,
                      kind: Kind,
                      out: Option<Value>,
                      tmps: &[Value],
                      inputs: &[uint],
                      successors: &[uint]) {
    match kind {
      Increment => self.put(out, inputs[0] + 1),
      AB => (), // nop
      JustUse => (), // nop
      Print => self.put(out, 0),
      Zero => self.put(out, 0),
      Ten => self.put(out, 10),
      Return => {
        self.result = Some(inputs[0]);
        return;
      },
      BranchIfBigger => {
        self.put(Some(tmps[0]), 0);
        if inputs[0] > inputs[1] {
          self.ip = successors[0];
        } else {
          self.ip = successors[1];
        }
        return;
      }
    }

    // Move forward
    self.ip += 1;
  }
}
