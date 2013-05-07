extern mod std;
use linearscan::graph::GraphBuilder;
mod linearscan;

#[deriving(Eq)]
enum Kind {
  Action0,
  Action1,
  Goto
}

#[test]
fn one_block_graph() {
  let mut g: ~GraphBuilder<Kind> = ~GraphBuilder::new();

  do g.block() |b| {
    let v = b.add(Action0, ~[]);
    b.add(Action1, ~[v]);
  };
}

#[test]
fn loop_graph() {
  let mut g: ~GraphBuilder<Kind> = ~GraphBuilder::new();

  let b0 = do g.block() |b| {
    b.add(Goto, ~[]);
  };

  let b1 = do g.block() |b| {
    b.add(Goto, ~[]);
    b.goto(b0);
  };

  do g.with_block(b0) |b| {
    b.goto(b1);
  }
}
