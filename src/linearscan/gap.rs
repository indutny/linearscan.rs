use linearscan::graph::{Graph, KindHelper, InstrId, GapState, GapAction,
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

impl<K: KindHelper+Copy+ToStr> GapResolver for Graph<K> {
  fn resolve_gaps(&mut self) {
    let mut keys = ~[];
    for self.gaps.each_key() |id| {
      keys.push(*id);
    }
    for keys.each() |id| {
      let state = self.resolve_gap(id);

      // Overwrite previous state
      self.gaps.insert(*id, state);
    }
  }
}

impl<K: KindHelper+Copy+ToStr> GapResolverHelper for Graph<K> {
  fn resolve_gap(&mut self, id: &InstrId) -> ~GapState {
    let state = self.gaps.pop(id).unwrap();
    let mut status = vec::from_elem(state.actions.len(), ToMove);

    let mut i = 0;
    let mut result = ~[];
    while i < state.actions.len() {
      let action = state.actions[i];
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
    let (from, to) = match actions[i] {
      Move(from, to) => (self.intervals.get(&from).value,
                         self.intervals.get(&to).value),
      _ => fail!("Expected move")
    };

    // Ignore nop moves
    if from == to { return false; }

    s[i] = Moving;
    let mut j = 0;
    let mut circular = false;
    let mut sentinel = false;
    while j < actions.len() {
      let other_from = match actions[j] {
        Move(from, _) => self.intervals.get(&from).value,
        _ => fail!("Expected move")
      };

      if other_from == to {
        match s[j] {
          ToMove => {
            let r = self.move_one(actions, j, s, result);
            if r {
              assert!(circular);
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
      match actions[i] {
        Move(from, to) => result.push(Swap(from, to)),
        _ => fail!("Expected move")
      }
    } else if !sentinel {
      result.push(copy actions[i]);
    }
    s[i] = Moved;

    return circular || sentinel;
  }
}
