use crate::entities::EntityIndex;

pub struct HierarchyComponent<T> {
    pub parent: Option<EntityIndex<T>>,
    pub children: Vec<EntityIndex<T>>,
}
