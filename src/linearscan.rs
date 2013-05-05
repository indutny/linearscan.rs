pub struct Config {
  register_count: uint
}

struct LiveRange {
  start: uint,
  end: uint
}

enum UseKind {
  UseAny,
  UseRegister
}

struct Use {
  pos: uint,
  kind: UseKind
}

pub enum Value {
  Virtual,
  Register(uint),
  Stack(uint)
}

pub struct Interval {
  ranges: ~[LiveRange],
  parent: Option<@Interval>,
  children: ~[@Interval],
  value: ~Value
}

impl Interval {
  pub fn new() -> @Interval {
    return @Interval {
      ranges: ~[],
      parent: None,
      children: ~[],
      value: ~Virtual
    }
  }
}

pub trait Block<I:Instruction> {
  fn instruction_count(&self) -> uint;
  fn instruction_at(&self, i: uint) -> @I;
  fn predecessor_count(&self) -> uint;
  fn predecessor_at(&self, i: uint) -> @Self;
  fn successor_count(&self) -> uint;
  fn successort_at(&self, i: uint) -> @Self;

  fn insert_move(&self, pos: uint, from: @Interval, to: @Interval);
}

pub trait Instruction {
  fn id(&self) -> uint;
  fn output(&self) -> @Interval;
  fn input_count(&self) -> uint;
  fn input_at(&self, i: uint) -> @Interval;
  fn is_move(&self) -> bool;
}

priv trait BlockHelper<I:Instruction> {
  fn start(&self) -> uint;
  fn end(&self) -> uint;
  fn each_instr(&self, f: &fn(&I) -> bool);
}

impl<I:Instruction, B:Block<I> > BlockHelper<I> for B {
  fn start(&self) -> uint {
    assert!(self.instruction_count() >= 1);
    return self.instruction_at(0).id();
  }

  fn end(&self) -> uint {
    assert!(self.instruction_count() >= 1);
    return self.instruction_at(self.instruction_count() - 1).id();
  }

  fn each_instr(&self, f: &fn(&I) -> bool) {
    let n = self.instruction_count();
    let mut i = 0;
    while i < n {
      if !f(self.instruction_at(i)) {
        break;
      }
      i += 1;
    }
  }
}

priv struct BlockWrapper<B> {
  is_loop: bool,
  loop_index: uint,
  loop_depth: uint,
  block: @B
}

pub struct Allocator {
  config: ~Config,
  loop_index: uint
}

pub impl<I:Instruction, B:Block<I> > Allocator {
  // Go through graph and create linear list of blocks
  priv fn flatten(&self, root: @B) -> ~[@BlockWrapper<B>] {
    let mut res: ~[@BlockWrapper<B>] = ~[];
    let mut queue = ~[root];

    while queue.len() > 0 {
    }

    return res;
  }

  priv fn run(&self, root: @B) {
    let list = self.flatten(root);
    assert!(list.len() >= 1);
  }

  fn run(root: @B, config: Config) {
    let a = ~Allocator {
      config: ~config,
      loop_index: 0
    };
    a.run(root);
  }
}
