use linearscan::{Graph, Generator, GeneratorFunctions, KindHelper,
                 UseKind, UseAny, UseRegister, UseFixed,
                 Value, RegisterVal, StackVal, GroupId, BlockId, InstrId};
use extra::smallintmap::SmallIntMap;

#[deriving(Eq, ToStr)]
pub enum Kind {
  Increment,
  Sum,
  DoubleSum,
  BranchIfBigger,
  JustUse,
  Nop,
  Print,
  Number(uint),
  DoubleNumber(float),
  ToDouble,
  Return,
  ReturnDouble
}

// Register groups
pub static Normal: GroupId = 0;
pub static Double: GroupId = 1;

impl KindHelper for Kind {
  fn clobbers(&self, _: GroupId) -> bool {
    match self {
      &Print => true,
      _ => false
    }
  }

  fn temporary(&self) -> ~[GroupId] {
    match self {
      &BranchIfBigger => ~[Normal],
      _ => ~[]
    }
  }

  fn use_kind(&self, i: uint) -> UseKind {
    match self {
      &BranchIfBigger if i == 0 => UseFixed(Normal, 2),
      &JustUse => UseFixed(Normal, 1),
      &Print => UseFixed(Normal, 3),
      &Return => UseFixed(Normal, 0),
      &ReturnDouble => UseFixed(Double, 0),
      &DoubleSum => UseRegister(Double),
      &ToDouble => UseRegister(Normal),
      _ => UseAny(Normal)
    }
  }

  fn result_kind(&self) -> Option<UseKind> {
    match self {
      &Return => None,
      &ReturnDouble => None,
      &BranchIfBigger => None,
      &JustUse => None,
      &Nop => None,
      &DoubleNumber(_) => Some(UseAny(Double)),
      &DoubleSum => Some(UseRegister(Double)),
      &ToDouble => Some(UseRegister(Double)),
      _ => Some(UseRegister(Normal))
    }
  }
}

pub struct Emulator {
  ip: InstrId,
  instructions: ~[Instruction],
  blocks: ~SmallIntMap<uint>,
  result: Option<Either<uint, float> >,
  registers: ~SmallIntMap<uint>,
  double_registers: ~SmallIntMap<float>,
  stack: ~SmallIntMap<uint>,
  double_stack: ~SmallIntMap<float>
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
      double_registers: ~SmallIntMap::new(),
      stack: ~SmallIntMap::new(),
      double_stack: ~SmallIntMap::new()
    }
  }

  fn run(&mut self, graph: &Graph<Kind>) -> Either<uint, float> {
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

  fn get(&self, slot: Value) -> Either<uint, float> {
    match slot {
      RegisterVal(Normal, r) => Left(*self.registers.find(&r)
                                          .expect("Defined register")),
      RegisterVal(Double, r) => Right(*self.double_registers.find(&r)
                                           .expect("Defined double register")),
      StackVal(Normal, s) => Left(*self.stack.find(&s)
                                       .expect("Defined stack slot")),
      StackVal(Double, s) => Right(*self.double_stack.find(&s)
                                        .expect("Defined double stack slot")),
      _ => fail!()
    }
  }

  fn put(&mut self, slot: Value, value: Either<uint, float>) {
    match slot {
      RegisterVal(Normal, r) => self.registers.insert(r, value.unwrap_left()),
      RegisterVal(Double, r) => self.double_registers
                                    .insert(r, value.unwrap_right()),
      StackVal(Normal, s) => self.stack.insert(s, value.unwrap_left()),
      StackVal(Double, s) => self.double_stack.insert(s, value.unwrap_right()),
      _ => fail!()
    };
  }

  fn exec_generic(&mut self, instr: &GenericInstruction) {
    let out = instr.output;
    let inputs = do instr.inputs.map() |i| { self.get(*i) };
    let tmp = copy instr.temporary;

    match instr.kind {
      Increment => self.put(out.expect("Increment out"),
                            Left(inputs[0].unwrap_left() + 1)),
      JustUse => (), // nop
      Nop => (), // nop
      Print => self.put(out.expect("Print out"), Left(0)),
      Number(n) => self.put(out.expect("Number out"), Left(n)),
      DoubleNumber(n) => self.put(out.expect("Double Number out"), Right(n)),
      Sum => self.put(out.expect("Sum out"),
                      Left(inputs[0].unwrap_left() + inputs[1].unwrap_left())),
      DoubleSum => self.put(out.expect("Double sum out"),
                            Right(inputs[0].unwrap_right() +
                                  inputs[1].unwrap_right())),
      ToDouble => self.put(out.expect("ToDouble out"),
                           Right(inputs[0].unwrap_left() as float)),
      Return => {
        assert!(inputs[0].is_left());
        self.result = Some(inputs[0]);
        return;
      },
      ReturnDouble => {
        assert!(inputs[0].is_right());
        self.result = Some(inputs[0]);
        return;
      },
      BranchIfBigger => {
        self.put(tmp[0], Left(0));
        if inputs[0].unwrap_left() > inputs[1].unwrap_left() {
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
