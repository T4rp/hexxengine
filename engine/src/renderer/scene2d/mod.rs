pub mod buffer_objects;
pub mod buffer_writing;
mod pipelines;
mod resources;

pub use buffer_objects::*;
pub use pipelines::*;
pub use resources::*;

pub const MAX_VERTICES_2D: usize = 50000;
