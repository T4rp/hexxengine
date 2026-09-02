mod camera;
mod part;

use thunderdome::Index;

pub type EntityIndex<T> = (T, Index);

pub use camera::Camera;
pub use part::Part;
