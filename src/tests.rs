use linearscan::GraphBuilder;
mod linearscan;

#[deriving(Eq)]
enum Kind {
  Action0,
  Action1
}

#[test]
fn one_block_graph() {
  let mut g: ~GraphBuilder<Kind> = GraphBuilder::new();

  do g.block() |b| {
    let v = b.add(Action0, ~[]);
    b.add(Action1, ~[v]);
    b.end();
  };

  io::println(fmt!("%?", g));
}
