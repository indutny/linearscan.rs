use linearscan::*;
use extra::smallintmap::SmallIntMap;

#[deriving(Eq, ToStr, Clone)]
pub enum Kind {
  Increment,
  Sum,
  DoubleSum,
  MultAdd,
  BranchIfBigger,
  JustUse,
  FixedUse,
  Nop,
  Print,
  Number(uint),
  DoubleNumber(float),
  ToDouble,
  Return,
  ReturnDouble
}

// Register groups
#[deriving(Clone, Eq, ToStr)]
pub enum Group {
  Normal,
  Double
}

// Registers
#[deriving(Clone, Eq, ToStr)]
pub enum Register {
  rax, rbx, rcx, rdx,
  xmm1, xmm2, xmm3, xmm4
}

impl GroupHelper for Group {
  fn any() -> Group { Normal }
  fn to_uint(&self) -> uint { *self as uint }
  fn from_uint(i: uint) -> Group {
    match i {
      0 => Normal,
      1 => Double,
      _ => fail!()
    }
  }
}

impl RegisterHelper<Group> for Register {
  fn group(&self) -> Group {
    match *self {
      rax => Normal, rbx => Normal, rcx => Normal, rdx => Normal,
      xmm1 => Double, xmm2 => Double, xmm3 => Double, xmm4 => Double
    }
  }

  fn to_uint(&self) -> uint {
    match self.group() {
      Normal => *self as uint,
      Double => *self as uint - 4
    }
  }

  fn from_uint(g: &Group, i: uint) -> Register {
    match g {
      &Normal => match i {
        0 => rax, 1 => rbx, 2 => rcx, 3 => rdx, _ => fail!()
      },
      &Double => match i {
        0 => xmm1, 1 => xmm2, 2 => xmm3, 3 => xmm4, _ => fail!()
      }
    }
  }
}

impl KindHelper<Group, Register> for Kind {
  fn clobbers(&self, _: &Group) -> bool {
    match self {
      &Print => true,
      _ => false
    }
  }

  fn temporary(&self) -> ~[Group] {
    match self {
      &BranchIfBigger => ~[Normal],
      _ => ~[]
    }
  }

  fn use_kind(&self, i: uint) -> UseKind<Group, Register> {
    match self {
      &BranchIfBigger if i == 0 => UseFixed(rcx),
      &JustUse => UseFixed(rbx),
      &FixedUse => {
        let r: Register = RegisterHelper::from_uint(&Normal, i);
        UseFixed(r)
      },
      &Print => UseFixed(rdx),
      &Return => UseFixed(rax),
      &ReturnDouble => UseFixed(xmm1),
      &DoubleSum => UseRegister(Double),
      &ToDouble => UseRegister(Normal),
      _ => UseAny(Normal)
    }
  }

  fn result_kind(&self) -> Option<UseKind<Group, Register> > {
    match self {
      &Return => None,
      &ReturnDouble => None,
      &BranchIfBigger => None,
      &JustUse => None,
      &FixedUse => None,
      &Nop => None,
      &DoubleNumber(_) => Some(UseAny(Double)),
      &DoubleSum => Some(UseRegister(Double)),
      &ToDouble => Some(UseRegister(Double)),
      _ => Some(UseRegister(Normal))
    }
  }
}

pub struct Emulator {
  ip: uint,
  instructions: ~[Instruction],
  blocks: ~SmallIntMap<uint>,
  result: Option<Either<uint, float> >,
  registers: ~SmallIntMap<uint>,
  double_registers: ~SmallIntMap<float>,
  stack: ~SmallIntMap<uint>,
  double_stack: ~SmallIntMap<float>
}

#[deriving(Clone)]
enum Instruction {
  Move(Value<Group, Register>, Value<Group, Register>),
  Swap(Value<Group, Register>, Value<Group, Register>),
  UnexpectedEnd,
  Block(BlockId),
  Goto(BlockId),
  Generic(GenericInstruction)
}

#[deriving(Clone)]
struct GenericInstruction {
  kind: Kind,
  output: Option<Value<Group, Register>>,
  inputs: ~[Value<Group, Register>],
  temporary: ~[Value<Group, Register>],
  succ: ~[BlockId]
}

impl GeneratorFunctions<Kind, Group, Register> for Emulator {
  fn prelude(&mut self) {
    // nop
  }

  fn epilogue(&mut self) {
    self.instructions.push(UnexpectedEnd);
  }

  fn swap(&mut self,
          left: &Value<Group, Register>,
          right: &Value<Group, Register>) {
    self.instructions.push(Swap(left.clone(), right.clone()));
  }

  fn move(&mut self,
          from: &Value<Group, Register>,
          to: &Value<Group, Register>) {
    self.instructions.push(Move(from.clone(), to.clone()));
  }

  fn block(&mut self, id: BlockId) {
    let ip = self.instructions.len();
    self.blocks.insert(id.to_uint(), ip);
    self.instructions.push(Block(id));
  }

  fn goto(&mut self, id: BlockId) {
    self.instructions.push(Goto(id));
  }

  fn instr(&mut self,
           kind: &Kind,
           output: Option<Value<Group, Register>>,
           inputs: &[Value<Group, Register>],
           temporary: &[Value<Group, Register>],
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

pub fn run_test(expected: Either<uint, float>,
                body: &fn(b: &mut Graph<Kind, Group, Register>)) {
  let mut g = ~Graph::new();

  body(&mut *g);

  g.allocate(Config {
    register_groups: ~[
      4, // normal registers
      4  // double registers
    ]
  }).get();

  let mut emu = Emulator::new();
  let got = emu.run(g);
  if got != expected {
    fail!(fmt!("got %? expected %?", got, expected));
  }
}

impl Emulator {
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

  fn run(&mut self,
         graph: &Graph<Kind, Group, Register>) -> Either<uint, float> {
    // Generate instructions
    graph.generate(self);

    let instructions = self.instructions.clone();
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
          let block_ip = self.blocks.find(&block.to_uint())
                                    .expect("Block to be present");
          self.ip = *block_ip;
        },
        Generic(ref instr) => self.exec_generic(instr)
      }
    }
  }

  fn get(&self, slot: Value<Group, Register>) -> Either<uint, float> {
    match slot {
      RegisterVal(r) if r.group() == Normal => {
        Left(*self.registers.find(&r.to_uint())
                  .expect("Defined register"))
      },
      RegisterVal(r) if r.group() == Double => {
        Right(*self.double_registers.find(&r.to_uint())
                   .expect("Defined double register"))
      },
      StackVal(Normal, s) => {
        Left(*self.stack.find(&s.to_uint())
                  .expect("Defined stack slot"))
      },
      StackVal(Double, s) => {
        Right(*self.double_stack.find(&s.to_uint())
                   .expect("Defined double stack slot"))
      },
      _ => fail!()
    }
  }

  fn put(&mut self, slot: Value<Group, Register>, value: Either<uint, float>) {
    match slot {
      RegisterVal(r) if r.group() == Normal => {
        self.registers.insert(r.to_uint(), value.unwrap_left())
      },
      RegisterVal(r) if r.group() == Double => {
        self.double_registers.insert(r.to_uint(), value.unwrap_right())
      },
      StackVal(Normal, s) => {
        self.stack.insert(s.to_uint(), value.unwrap_left())
      },
      StackVal(Double, s) => {
        self.double_stack.insert(s.to_uint(), value.unwrap_right())
      },
      _ => fail!()
    };
  }

  fn exec_generic(&mut self, instr: &GenericInstruction) {
    let out = instr.output;
    let inputs = instr.inputs.map(|i| self.get(*i));
    let tmp = instr.temporary.clone();

    match instr.kind {
      Increment => self.put(out.expect("Increment out"),
                            Left(inputs[0].unwrap_left() + 1)),
      JustUse => (), // nop
      FixedUse => (), // nop
      Nop => (), // nop
      Print => self.put(out.expect("Print out"), Left(0)),
      Number(n) => self.put(out.expect("Number out"), Left(n)),
      DoubleNumber(n) => self.put(out.expect("Double Number out"), Right(n)),
      Sum => self.put(out.expect("Sum out"),
                      Left(inputs[0].unwrap_left() + inputs[1].unwrap_left())),
      MultAdd => self.put(out.expect("Mult add out"),
                          Left(inputs[0].unwrap_left() *
                                 inputs[1].unwrap_left() +
                               inputs[2].unwrap_left())),
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
          self.ip = *self.blocks.find(&instr.succ[0].to_uint())
                                .expect("branch true");
        } else {
          self.ip = *self.blocks.find(&instr.succ[1].to_uint())
                                .expect("branch false");
        }
        return;
      }
    }

    // Move forward
    self.ip += 1;
  }
}
