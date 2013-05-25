extern mod extra;

use linearscan::{Allocator, Generator, GeneratorFunctions,
                 Config, Graph, BlockId};
use extra::json::ToJson;
use emulator::*;

#[path="../src/linearscan.rs"]
mod linearscan;
mod emulator;

fn graph_test(expected: uint, body: &fn(b: &mut Graph<Kind>)) {
  let mut g = ~Graph::new::<Kind>();

  body(&mut *g);

  g.allocate(Config { register_count: 4 }).get();

  let mut emu = Emulator::new();
  assert!(emu.run(g) == expected);
}

#[test]
fn realword_example() {
  do graph_test(21) |g| {
    let phi = g.phi();

    let cond = g.empty_block();
    let left = g.empty_block();
    let after_left = g.empty_block();
    let right = g.empty_block();
    let ret = g.new_instr(Number(10), ~[]);

    do g.block() |b| {
      b.make_root();

      b.add_existing(ret);
      let zero = b.add(Number(0), ~[]);
      b.to_phi(zero, phi);
      b.goto(cond);
    };

    do g.with_block(cond) |b| {
      let ten = b.add(Number(10), ~[]);
      b.add(JustUse, ~[phi]);
      b.add(BranchIfBigger, ~[phi, ten]);
      b.branch(right, left);
    };

    do g.with_block(left) |b| {
      let print_res = b.add(Print, ~[phi]);
      b.add(Increment, ~[print_res]);
      b.goto(after_left);
    };

    do g.with_block(after_left) |b| {
      let counter = b.add(Increment, ~[phi]);
      b.to_phi(counter, phi);
      b.goto(cond);
    };

    do g.with_block(right) |b| {
      let sum = b.add(Sum, ~[ret, phi]);
      b.add(Return, ~[sum]);
      b.end();
    };
  };
}

#[test]
fn nested_loops() {
  do graph_test(0) |g| {
    fn create_loop(g: &mut Graph<Kind>,
                   f: &fn(&mut Graph<Kind>) -> Option<(BlockId, BlockId)>)
        -> Option<(BlockId, BlockId)> {
      let phi = g.phi();
      let cond = g.empty_block();
      let body = g.empty_block();
      let after = g.empty_block();

      // Pre
      let pre = do g.block() |b| {
        let init = b.add(Number(0), ~[]);
        b.to_phi(init, phi);
        b.goto(cond);
      };

      // Cond
      do g.with_block(cond) |b| {
        let limit = b.add(Number(4), ~[]);
        b.add(BranchIfBigger, ~[phi, limit]);
        b.branch(after, body);
      };

      // Body
      do g.with_block(body) |b| {
        let next = b.add(Increment, ~[phi]);
        b.to_phi(next, phi);
      };

      do g.with_block(after) |b| {
        b.add(Nop, ~[]);
      };

      match f(g) {
        // Link loops together
        Some((pre, after)) => {
          do g.with_block(body) |b| {
            b.goto(pre);
          };
          do g.with_block(after) |b| {
            b.goto(cond);
          };
        },
        // Just loop
        None => {
          do g.with_block(body) |b| {
            b.goto(cond);
          };
        }
      };

      Some((pre, after))
    }

    let (pre, after) = do create_loop(g) |g| {
      do create_loop(g) |_| {
        None
      }
    }.unwrap();

    g.set_root(pre);

    do g.with_block(after) |b| {
      let num = b.add(Number(0), ~[]);
      b.add(Return, ~[num]);
      b.end();
    };
  };
}
