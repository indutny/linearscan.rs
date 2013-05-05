use linearscan::Interval;
mod linearscan;

struct Block {
  instructions: ~[@Instruction],
  predecessors: ~[@Block],
  successors: ~[@Block]
}

struct Instruction {
  id: uint,
  kind: Kind,
  inputs: ~[@Interval],
  output: @Interval
}

#[deriving(Eq)]
enum Kind {
  Action0,
  Action1,
  Move
}

impl linearscan::Instruction for Instruction {
  fn id(&self) -> uint {
    return self.id;
  }

  fn output(&self) -> @Interval {
    return self.output;
  }

  fn input_count(&self) -> uint {
    return self.inputs.len();
  }

  fn input_at(&self, i: uint) -> @Interval {
    return self.inputs[i];
  }

  fn is_move(&self) -> bool {
    return self.kind == Move;
  }
}

impl linearscan::Block<Instruction> for Block {
  fn instruction_count(&self) -> uint {
    return self.instructions.len();
  }

  fn instruction_at(&self, i: uint) -> @Instruction {
    return self.instructions[i];
  }

  fn predecessor_count(&self) -> uint {
    return self.predecessors.len();
  }

  fn predecessor_at(&self, i: uint) -> @Block {
    return self.predecessors[i];
  }

  fn successor_count(&self) -> uint {
    return self.successors.len();
  }

  fn successort_at(&self, i: uint) -> @Block {
    return self.successors[i];
  }

  fn insert_move(&self, pos: uint, from: @Interval, to: @Interval) {
    fail!(fmt!("Not implemented yet %? %? %?", pos, from, to));
  }
}

impl Instruction {
  fn new(id: uint, kind: Kind, inputs: ~[@Interval]) -> @Instruction {
    return @Instruction {
      id: id,
      kind: kind,
      inputs: inputs,
      output: Interval::new()
    };
  }
}

#[test]
fn allocation_test() {
  let mut instr = ~[];

  let act0 = Instruction::new(0, Action0, ~[]);
  instr.push(act0);
  instr.push(Instruction::new(2, Action1, ~[act0.output]));

  let mut root = @Block {
    instructions: instr,
    successors: ~[],
    predecessors: ~[]
  };

  linearscan::Allocator::run(root, linearscan::Config {
    register_count: 4
  });
}
