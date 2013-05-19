extern mod std;

use linearscan::{Allocator, Config, Graph, KindHelper,
                 UseKind, UseAny, UseRegister};
use std::json::ToJson;
mod linearscan;

#[deriving(Eq, ToStr)]
enum Kind {
  Phi,
  Increment,
  BranchIfBigger,
  PrintHello,
  Zero,
  Ten,
  Goto,
  Return
}

impl KindHelper for Kind {
  fn is_call(&self) -> bool {
    match self {
      &PrintHello => true,
      _ => false
    }
  }

  fn tmp_count(&self) -> uint {
    match self {
      &BranchIfBigger => 1,
      _ => 0
    }
  }

  fn use_kind(&self, _: uint) -> UseKind {
    UseAny
  }

  fn result_kind(&self) -> Option<UseKind> {
    match self {
      &Goto => None,
      &Return => None,
      &BranchIfBigger => None,
      &PrintHello => None,
      _ => Some(UseRegister)
    }
  }
}

fn graph_test(body: &fn(b: &mut Graph<Kind>)) {
  let mut g = ~Graph::new::<Kind>();

  body(&mut *g);

  g.allocate(Config { register_count: 4 });
  io::println(g.to_json().to_str());
}

#[test]
fn realword_example() {
  do graph_test() |g| {
    let phi = g.new_instr(Phi, ~[]);

    let start = do g.block() |b| {
      b.make_root();

      let zero = b.add(Zero, ~[]);
      b.add_arg(phi, zero);
    };

    let left = do g.block() |b| {
      let counter = b.add(Increment, ~[phi]);
      b.add_arg(phi, counter);
      b.add(Goto, ~[]);
    };

    let right = do g.block() |b| {
      b.add(PrintHello, ~[]);
      b.add(Return, ~[]);
      b.end();
    };

    let cond = do g.block() |b| {
      b.add_existing(phi);
      let ten = b.add(Ten, ~[]);
      b.add(BranchIfBigger, ~[phi, ten]);
      b.branch(left, right);
    };

    do g.with_block(left) |b| {
      b.goto(cond);
    };

    do g.with_block(start) |b| {
      b.goto(cond);
    };
  };
}
