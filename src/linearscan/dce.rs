use linearscan::graph::{Graph, KindHelper, InstrKind,
                        Phi, ToPhi, Gap, User};
use extra::bitv::BitvSet;

/// DCE = Dead Code Elimination
pub trait DCE<K> {
  fn eliminate_dead_code(&mut self);
}

pub trait DCEKindHelper {
  fn has_sideeffects(&self) -> bool;
}

impl<K: KindHelper+DCEKindHelper+Copy> DCE<K> for Graph<K> {
  fn eliminate_dead_code(&mut self) {
    // Get list of alive instructions
    let mut alive = ~BitvSet::new();
    let mut work_list = ~[];
    for self.instructions.each() |_, instr| {
      if instr.kind.has_sideeffects() {
        work_list.push(instr.id);
      }
    }

    while work_list.len() > 0 {
      let cur = work_list.shift();
      if !alive.insert(cur.to_uint()) { loop; }

      // Schedule inputs
      for self.get_instr(&cur).inputs.each() |&id| {
        work_list.push(id);
      }
    }

    // Filter out dead instructions in blocks
    for self.blocks.mutate_values() |_, block| {
      block.instructions.retain(|id| alive.contains(&id.to_uint()));
    }

    // And globally
    let mut queue = ~[];
    for self.instructions.each() |id, _| {
      if !alive.contains(id) { queue.push(*id) }
    }

    while queue.len() > 0 {
      self.instructions.pop(&queue.shift());
    }
  }
}

impl<K: DCEKindHelper+Copy> DCEKindHelper for InstrKind<K> {
  fn has_sideeffects(&self) -> bool {
    match self {
      &Phi(_) => false,
      &ToPhi(_) => false,
      &Gap => fail!("DCE should run before allocation"),
      &User(ref k) => k.has_sideeffects()
    }
  }
}
