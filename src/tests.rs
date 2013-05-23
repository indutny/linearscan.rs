extern mod extra;

use linearscan::{Allocator, Config, Graph, KindHelper,
                 UseKind, UseAny, UseRegister, UseFixed};
use extra::json::ToJson;
mod linearscan;

#[deriving(Eq, ToStr)]
enum Kind {
  Phi,
  Increment,
  BranchIfBigger,
  AB,
  JustUse,
  Print,
  Zero,
  Ten,
  Goto,
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
      &AB => UseFixed(i),
      &JustUse => UseFixed(1),
      &Print => UseFixed(3),
      &Return => UseFixed(0),
      _ => UseAny
    }
  }

  fn result_kind(&self) -> Option<UseKind> {
    match self {
      &Goto => None,
      &Return => None,
      &BranchIfBigger => None,
      &JustUse => None,
      &AB => None,
      _ => Some(UseRegister)
    }
  }
}

fn graph_test(body: &fn(b: &mut Graph<Kind>)) {
  let mut g = ~Graph::new::<Kind>();

  body(&mut *g);

  g.allocate(Config { register_count: 4 }).get();
  let writer = io::file_writer(&Path("./1.json"), [io::Create, io::Truncate]);
  match writer {
    Ok(writer) => writer.write_str(g.to_json().to_str()),
    Err(_) => ()
  };
}

#[test]
fn realword_example() {
  do graph_test() |g| {
    let phi = g.phi();

    let cond = g.empty_block();
    let left = g.empty_block();
    let after_left = g.empty_block();
    let right = g.empty_block();
    let ret = g.new_instr(Zero, ~[]);

    do g.block() |b| {
      b.make_root();

      b.add_existing(ret);
      let zero = b.add(Zero, ~[]);
      b.to_phi(zero, phi);
      b.add(Goto, ~[]);
      b.goto(cond);
    };

    do g.with_block(cond) |b| {
      let ten = b.add(Ten, ~[]);
      b.add(JustUse, ~[phi]);
      b.add(BranchIfBigger, ~[phi, ten]);
      b.branch(left, right);
    };

    do g.with_block(left) |b| {
      let print_res = b.add(Print, ~[phi]);
      b.add(Increment, ~[print_res]);
      b.add(Goto, ~[]);
      b.goto(after_left);
    };

    do g.with_block(after_left) |b| {
      let counter = b.add(Increment, ~[phi]);
      b.to_phi(counter, phi);
      b.add(Goto, ~[]);
      b.goto(cond);
    };

    do g.with_block(right) |b| {
      b.add(Return, ~[ret]);
      b.end();
    };
  };
}

// #[test]
fn ab_ba() {
  do graph_test() |g| {
    do g.block() |b| {
      b.make_root();

      let ten = b.add(Ten, ~[]);
      let zero = b.add(Zero, ~[]);
      b.add(AB, ~[ten, zero]);
      b.add(AB, ~[zero, ten]);
      b.end();
    };
  };
}
