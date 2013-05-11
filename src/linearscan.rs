pub use linearscan::graph::GraphBuilder;
pub use linearscan::allocator::{Allocator, Config};

#[path="linearscan/flatten.rs"]
mod flatten;

#[path="linearscan/allocator.rs"]
mod allocator;

#[path="linearscan/graph.rs"]
mod graph;
