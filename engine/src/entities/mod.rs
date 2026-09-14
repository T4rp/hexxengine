mod camera;
mod part;
mod selection_box;
mod transform_handles;

use thunderdome::Index;

pub type EntityIndex<T> = (T, Index);

pub use camera::Camera;
pub use part::Part;
pub use selection_box::SelectionBox;
pub use transform_handles::{TransformHandles, TransformType};
