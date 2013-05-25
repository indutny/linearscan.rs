use linearscan::{Graph, Generator, GeneratorFunctions, KindHelper,
                 UseKind, UseAny, UseRegister, UseFixed,
                 Value, Register, Stack, BlockId, InstrId};
use extra::smallintmap::SmallIntMap;

#[deriving(Eq, ToStr)]
pub enum Kind {
  Increment,
  Sum,
  BranchIfBigger,
  JustUse,
  Nop,
  Print,
  Number(uint),
  Return
}

impl KindHelper for Kind {
  fn is_call(&self) -> bool {
    match self {
      &Print => true,
      _ => false
    }
  }

  fn tmp_count(&self) -> uint {
    match self {
      &BranchIfBigger => 1,
      _ => 0
    }
  }

  fn use_kind(&self, i: uint) -> UseKind {
    match self {
      &BranchIfBigger if i == 0 => UseFixed(2),
      &JustUse => UseFixed(1),
      &Print => UseFixed(3),
      &Return => UseFixed(0),
      _ => UseAny
    }
  }

  fn result_kind(&self) -> Option<UseKind> {
    match self {
      &Return => None,
      &BranchIfBigger => None,
      &JustUse => None,
      _ => Some(UseRegister)
    }
  }
}

pub struct Emulator {
  ip: InstrId,
  instructions: ~[Instruction],
  blocks: ~SmallIntMap<uint>,
  result: Option<uint>,
  registers: ~SmallIntMap<uint>,
  stack: ~SmallIntMap<uint>
}

enum Instruction {
  Move(Value, Value),
  Swap(Value, Value),
  UnexpectedEnd,
  Block(BlockId),
  Goto(BlockId),
  Generic(GenericInstruction)
}

struct GenericInstruction {
  kind: Kind,
  output: Option<Value>,
  inputs: ~[Value],
  temporary: ~[Value],
  succ: ~[BlockId]
}

impl GeneratorFunctions<Kind> for Emulator {
  fn prelude(&mut self) {
    // nop
  }

  fn epilogue(&mut self) {
    self.instructions.push(UnexpectedEnd);
  }

  fn swap(&mut self, left: Value, right: Value) {
    self.instructions.push(Swap(left, right));
  }

  fn move(&mut self, from: Value, to: Value) {
    self.instructions.push(Move(from, to));
  }

  fn block(&mut self, id: BlockId) {
    let ip = self.instructions.len();
    self.blocks.insert(id, ip);
    self.instructions.push(Block(id));
  }

  fn goto(&mut self, id: BlockId) {
    self.instructions.push(Goto(id));
  }

  fn instr(&mut self,
           kind: &Kind,
           output: Option<Value>,
           inputs: &[Value],
           temporary: &[Value],
           succ: &[BlockId]) {
    self.instructions.push(Generic(GenericInstruction {
      kind: *kind,
      output: output,
      inputs: inputs.to_owned(),
      temporary: temporary.to_owned(),
      succ: succ.to_owned()
    }));
  }
}

pub impl Emulator {
  fn new() -> Emulator {
    Emulator {
      ip: 0,
      result: None,
      instructions: ~[],
      blocks: ~SmallIntMap::new(),
      registers: ~SmallIntMap::new(),
      stack: ~SmallIntMap::new()
    }
  }

  fn run(&mut self, graph: &Graph<Kind>) -> uint {
    // Generate instructions
    graph.generate(self);

    let instructions = copy self.instructions;
    loop {
      // Execution finished
      if self.result.is_some() {
        return self.result.unwrap();
      }

      match instructions[self.ip] {
        UnexpectedEnd => fail!("This end was really unexpected"),
        Block(_) => { self.ip += 1; },
        Move(from, to) => {
          let v = self.get(from);
          self.put(to, v);
          self.ip += 1;
        },
        Swap(left, right) => {
          let t = self.get(left);
          let v = self.get(right);
          self.put(left, v);
          self.put(right, t);
          self.ip += 1;
        },
        Goto(block) => {
          let block_ip = self.blocks.find(&block).expect("Block to be present");
          self.ip = *block_ip;
        },
        Generic(ref instr) => self.exec_generic(instr)
      }
    }
  }

  fn get(&self, slot: Value) -> uint {
    match slot {
      Register(r) => *self.registers.find(&r).expect("Defined register"),
      Stack(s) => *self.stack.find(&s).expect("Defined stack slot"),
      _ => fail!()
    }
  }

  fn put(&mut self, slot: Value, value: uint) {
    match slot {
      Register(r) => { self.registers.insert(r, value); },
      Stack(s) => { self.stack.insert(s, value); },
      _ => fail!()
    }
  }

  fn exec_generic(&mut self, instr: &GenericInstruction) {
    let out = instr.output;
    let inputs = do instr.inputs.map() |i| { self.get(*i) };
    let tmp = copy instr.temporary;

    match instr.kind {
      Increment => self.put(out.expect("Increment out"), inputs[0] + 1),
      JustUse => (), // nop
      Nop => (), // nop
      Print => self.put(out.expect("Print out"), 0),
      Number(n) => self.put(out.expect("Number out"), n),
      Sum => self.put(out.expect("Sum out"), inputs[0] + inputs[1]),
      Return => {
        self.result = Some(inputs[0]);
        return;
      },
      BranchIfBigger => {
        self.put(tmp[0], 0);
        if inputs[0] > inputs[1] {
          self.ip = *self.blocks.find(&instr.succ[0]).expect("branch true");
        } else {
          self.ip = *self.blocks.find(&instr.succ[1]).expect("branch false");
        }
        return;
      }
    }

    // Move forward
    self.ip += 1;
  }
}
