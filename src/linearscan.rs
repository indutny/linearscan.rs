pub use linearscan::graph::{Graph, KindHelper,
                            UseKind, UseAny, UseRegister, UseFixed,
                            Virtual, Register, Stack};
pub use linearscan::allocator::{Allocator, Config};

#[path="linearscan/flatten.rs"]
mod flatten;

#[path="linearscan/liveness.rs"]
mod liveness;

#[path="linearscan/allocator.rs"]
mod allocator;

#[path="linearscan/graph.rs"]
mod graph;
