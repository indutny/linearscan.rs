extern mod std;

use linearscan::{Allocator, Config, Graph, KindHelper, UseKind, UseAny};
mod linearscan;

#[deriving(Eq)]
enum Kind {
  Action0,
  Action1,
  Goto,
  Return
}

impl KindHelper for Kind {
  fn is_call(&self) -> bool {
    false
  }

  fn tmp_count(&self) -> uint {
    match self {
      &Action0 => 1,
      _ => 0
    }
  }

  fn use_kind(&self, _: uint) -> UseKind {
    UseAny
  }
}

fn graph_test(body: &fn(b: &mut Graph<Kind>)) {
  let mut g = ~Graph::new::<Kind>();

  body(&mut *g);

  g.allocate(Config { register_count: 4 });
}

#[test]
fn one_block_graph() {
  do graph_test() |g| {
    do g.block() |b| {
      b.make_root();

      let v = b.add(Action0, ~[]);
      b.add(Action1, ~[v]);
    };
  };
}

#[test]
fn loop_graph() {
  do graph_test() |g| {
    let b0 = do g.block() |b| {
      b.make_root();

      b.add(Goto, ~[]);
    };

    let b1 = do g.block() |b| {
      b.add(Goto, ~[]);
      b.goto(b0);
    };

    do g.with_block(b0) |b| {
      b.goto(b1);
    }
  };
}

#[test]
fn nested_loops() {
  do graph_test() |g| {
    // 0 -> 1 -> 2 -> 0
    // 1 -> 2 -> 1
    // 1 -> (3)
    let b0 = do g.block() |b| {
      b.make_root();
      b.add(Goto, ~[]);
    };

    let b1 = do g.block() |b| {
      b.add(Goto, ~[]);
    };

    let b2 = do g.block() |b| {
      b.add(Goto, ~[]);
      b.branch(b0, b1);
    };

    let b3 = do g.block() |b| {
      b.add(Return, ~[]);
    };

    do g.with_block(b0) |b| {
      b.goto(b1);
    };

    do g.with_block(b1) |b| {
      b.branch(b2, b3);
    };
  };
}
