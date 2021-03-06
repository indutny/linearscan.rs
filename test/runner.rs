extern mod extra;

use extra::json::ToJson;
use std::iterator;
use linearscan::*;
use emulator::*;

#[path="../src/linearscan.rs"]
mod linearscan;
mod emulator;

#[test]
fn realword_example() {
  do run_test(Left(21)) |g| {
    let phi = g.phi(Normal);

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

  do run_test(Left(125)) |g| {
    fn create_loop(g: &mut Graph<Kind, Group, Register>,
                   inp: InstrId,
                   f: &fn(&mut Graph<Kind,
                          Group, Register>,
                          inp: InstrId) -> Option<LoopResult>)
        -> Option<LoopResult> {
      let phi = g.phi(Normal);
      let res_phi = g.phi(Normal);
      let cond = g.empty_block();
      let body = g.empty_block();
      let after = g.empty_block();

      // Pre
      let pre = do g.block() |b| {
        let init = b.add(Number(0), ~[]);
        b.to_phi(init, phi);
        b.to_phi(inp, res_phi);
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
            b.goto(pre);
          };
          do g.with_block(after) |b| {
            b.to_phi(out, res_phi);
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

    let inp = g.new_instr(Number(0), ~[]);
    let LoopResult{ pre, after, out } = do create_loop(g, inp) |g, inp| {
      do create_loop(g, inp) |g, inp| {
        do create_loop(g, inp) |_, _| { None }
      }
    }.unwrap();

    // Start
    do g.block() |b| {
      b.make_root();
      b.add_existing(inp);
      b.goto(pre);
    };

    do g.with_block(after) |b| {
      b.add(Return, ~[out]);
      b.end();
    };
  };
}

#[test]
fn double_and_normal() {
  do run_test(Right(286.875)) |g| {
    do g.block() |b| {
      b.make_root();

      // Create very high register pressure
      let mut normals = ~[];
      let mut doubles = ~[];
      let count = 16;
      for i in iterator::range(0, count) {
        normals.push(b.add(Number(i + 1), ~[]));
        doubles.push(b.add(DoubleNumber(((i + 1) as float) / 8f), ~[]));
      }

      let mut total = b.add(DoubleNumber(0f), ~[]);
      for i in iterator::range(count - 1, 0).reverse_() {
        let left = b.add(Sum, ~[normals[i - 1], normals[i]]);
        let right = b.add(DoubleSum, ~[doubles[i - 1], doubles[i]]);
        let double_left = b.add(ToDouble, ~[left]);

        let subtotal = b.add(DoubleSum, ~[double_left, right]);
        total = b.add(DoubleSum, ~[total, subtotal]);
      }
      b.add(ReturnDouble, ~[total]);
      b.end();
    };
  };
}

#[test]
fn parallel_move_cycles() {
  do run_test(Left(1234)) |g| {
    do g.block() |b| {
      b.make_root();

      let n1 = b.add(Number(1), ~[]);
      let n2 = b.add(Number(2), ~[]);
      let n3 = b.add(Number(3), ~[]);
      let n4 = b.add(Number(4), ~[]);

      // 1 <=> 2
      b.add(FixedUse, ~[n1, n2, n3, n4]);
      b.add(FixedUse, ~[n2, n1, n3, n4]);

      // 1 <=> 2, 3 <=> 4
      b.add(FixedUse, ~[n1, n2, n3, n4]);
      b.add(FixedUse, ~[n2, n1, n4, n3]);

      // shift
      b.add(FixedUse, ~[n1, n2, n3, n4]);
      b.add(FixedUse, ~[n4, n1, n2, n3]);

      // reverse shift
      b.add(FixedUse, ~[n1, n2, n3, n4]);
      b.add(FixedUse, ~[n2, n3, n4, n1]);

      // mixed
      b.add(FixedUse, ~[n1, n2, n3, n4]);
      b.add(FixedUse, ~[n3, n2, n4, n1]);

      let ten = b.add(Number(10), ~[]);
      let mut res = b.add(Number(0), ~[]);
      res = b.add(MultAdd, ~[res, ten, n1]);
      res = b.add(MultAdd, ~[res, ten, n2]);
      res = b.add(MultAdd, ~[res, ten, n3]);
      res = b.add(MultAdd, ~[res, ten, n4]);

      b.add(Return, ~[res]);
      b.end();
    };
  };
}
