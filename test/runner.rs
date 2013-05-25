extern mod extra;

use linearscan::{Allocator, Generator, GeneratorFunctions,
                 Config, Graph, InstrId, BlockId};
use extra::json::ToJson;
use emulator::*;

#[path="../src/linearscan.rs"]
mod linearscan;
mod emulator;

fn graph_test(expected: uint, body: &fn(b: &mut Graph<Kind>)) {
  let mut g = ~Graph::new::<Kind>();

  body(&mut *g);

  g.allocate(Config { register_count: 4 }).get();


  let writer = io::file_writer(&Path("./1.json"), [io::Create, io::Truncate]);
  match writer {
    Ok(writer) => writer.write_str(g.to_json().to_str()),
    Err(_) => ()
  };
  let mut emu = Emulator::new();
  let got = emu.run(g);
  if got != expected {
    fail!(fmt!("got %? expected %?", got, expected));
  }
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
  struct LoopResult {
    pre: BlockId,
    after: BlockId,
    out: InstrId
  }

  do graph_test(25) |g| {
    fn create_loop(g: &mut Graph<Kind>,
                   in: InstrId,
                   f: &fn(&mut Graph<Kind>, in: InstrId) -> Option<LoopResult>)
        -> Option<LoopResult> {
      let phi = g.phi();
      let res_phi = g.phi();
      let cond = g.empty_block();
      let body = g.empty_block();
      let after = g.empty_block();

      // Pre
      let pre = do g.block() |b| {
        let init = b.add(Number(0), ~[]);
        b.to_phi(init, phi);
        b.to_phi(in, res_phi);
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

      match f(g, res_phi) {
        // Link loops together
        Some(LoopResult {pre, after, out}) => {
          do g.with_block(body) |b| {
            b.to_phi(out, res_phi);
            b.goto(pre);
          };
          do g.with_block(after) |b| {
            b.goto(cond);
          };
        },
        // Just loop
        None => {
          do g.with_block(body) |b| {
            let next = b.add(Increment, ~[res_phi]);
            b.to_phi(next, res_phi);
            b.goto(cond);
          };
        }
      };

      Some(LoopResult{ pre: pre, after: after, out: res_phi })
    }

    let in = g.new_instr(Number(0), ~[]);
    let LoopResult{ pre, after, out } = do create_loop(g, in) |g, in| {
      do create_loop(g, in) |_, _| { None }
    }.unwrap();

    // Start
    do g.block() |b| {
      b.make_root();
      b.add_existing(in);
      b.goto(pre);
    };

    do g.with_block(after) |b| {
      b.add(Return, ~[out]);
      b.end();
    };
  };
}
