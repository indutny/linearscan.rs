use linearscan::graph::{Graph, KindHelper, InstrKind,
                        Phi, ToPhi, Gap, User};

/// DCE = Dead Code Elimination
pub trait DCE<K> {
  fn eliminate_dead_code(&mut self);
}

pub trait DCEKindHelper {
  fn has_sideeffects(&self) -> bool;
}

trait DCEHelper<K> {
}

impl<K: KindHelper+DCEKindHelper+Copy> DCE<K> for Graph<K> {
  fn eliminate_dead_code(&mut self) {
  }
}

impl<K: KindHelper+DCEKindHelper+Copy> DCEHelper<K> for Graph<K> {
}

impl<K: DCEKindHelper+Copy> DCEKindHelper for InstrKind<K> {
  fn has_sideeffects(&self) -> bool {
    match self {
      &Phi(_) => false,
      &ToPhi(_) => false,
      &Gap => fail!("DCE should run before allocation"),
      &User(k) => k.has_sideeffects()
    }
  }
}
