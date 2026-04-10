mod buffer_objects;
mod pipelines;
mod resources;

pub use buffer_objects::*;
pub use pipelines::*;
pub use resources::*;

pub const SHADOW_MAP_RESOLUTION: u32 = 2048;
pub const MAX_INSTANCE_COUNT: usize = 10000;
