use std::vec;
use linearscan::*;
use linearscan::graph::{Graph, InstrId, GapState, GapAction,
                        Move, Swap};

#[deriving(Eq)]
enum MoveStatus {
  ToMove,
  Moving,
  Moved
}

pub trait GapResolver {
  fn resolve_gaps(&mut self);
}

trait GapResolverHelper {
  fn resolve_gap(&mut self, id: &InstrId) -> ~GapState;
  fn move_one(&mut self,
              actions: &[GapAction],
              i: uint,
              s: &mut [MoveStatus],
              result: &mut ~[GapAction]) -> bool;
}

impl<G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R>+Clone> GapResolver for Graph<K, G, R> {
  fn resolve_gaps(&mut self) {
    let mut keys = ~[];
    for self.gaps.each_key() |id| {
      keys.push(InstrId(*id));
    }
    for keys.each() |id| {
      let state = self.resolve_gap(id);

      // Overwrite previous state
      self.gaps.insert(id.to_uint(), state);
    }
  }
}

impl<G: GroupHelper,
     R: RegisterHelper<G>,
     K: KindHelper<G, R>+Clone> GapResolverHelper for Graph<K, G, R> {
  fn resolve_gap(&mut self, id: &InstrId) -> ~GapState {
    let state = self.gaps.pop(&id.to_uint()).unwrap();
    let mut status = vec::from_elem(state.actions.len(), ToMove);

    let mut i = 0;
    let mut result = ~[];
    while i < state.actions.len() {
      if status[i] == ToMove {
        self.move_one(state.actions, i, status, &mut result);
      }
      i += 1;
    }
    ~GapState { actions: result }
  }

  fn move_one(&mut self,
              actions: &[GapAction],
              i: uint,
              s: &mut [MoveStatus],
              result: &mut ~[GapAction]) -> bool {
    assert!(actions[i].kind == Move);
    let from = self.get_interval(&actions[i].from).value.clone();
    let to = self.get_interval(&actions[i].to).value.clone();

    // Ignore nop moves
    if from == to { return false; }

    s[i] = Moving;
    let mut j = 0;
    let mut circular = false;
    let mut sentinel = false;
    while j < actions.len() {
      assert!(actions[j].kind == Move);
      let other_from = self.get_interval(&actions[j].from).value.clone();

      if other_from == to {
        match s[j] {
          ToMove => {
            let r = self.move_one(actions, j, s, result);
            if r {
              assert!(!circular);
              circular = true;
            }
          },
          Moving => {
            sentinel = true;
          },
          Moved => ()
        }
      }

      j += 1;
    }

    if circular {
      result.push(GapAction {
        kind: Swap,
        from: actions[i].from,
        to: actions[i].to
      });
    } else if !sentinel {
      result.push(copy actions[i]);
    }
    s[i] = Moved;

    return circular || sentinel;
  }
}
