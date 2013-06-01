pub use linearscan::graph::{Graph, KindHelper,
                            UseKind, UseAny, UseRegister, UseFixed,
                            GroupId, BlockId, InstrId,
                            Value, RegisterVal, StackVal};
pub use linearscan::allocator::{Allocator, Config};
pub use linearscan::dce::{DCE, DCEKindHelper};
pub use linearscan::generator::{Generator, GeneratorFunctions};

#[path="linearscan/allocator.rs"]
mod allocator;

#[path="linearscan/dce.rs"]
mod dce;

#[path="linearscan/flatten.rs"]
mod flatten;

#[path="linearscan/gap.rs"]
mod gap;

#[path="linearscan/generator.rs"]
mod generator;

#[path="linearscan/graph.rs"]
mod graph;

#[path="linearscan/json.rs"]
mod json;

#[path="linearscan/liveness.rs"]
mod liveness;
