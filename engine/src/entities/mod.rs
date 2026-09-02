mod camera;
mod part;
mod selection_box;

use thunderdome::Index;

pub type EntityIndex<T> = (T, Index);

pub use camera::Camera;
pub use part::Part;
pub use selection_box::SelectionBox;
