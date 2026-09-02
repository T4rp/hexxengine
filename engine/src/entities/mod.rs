mod part;

use thunderdome::Index;

pub type EntityIndex<T> = (T, Index);

pub use part::Part;
