use hexxengine::thunderdome::Index;

type EntityIndex<T> = (T, Index);

pub struct HierarchyComponent<T> {
    pub parent: Option<EntityIndex<T>>,
    pub children: Vec<EntityIndex<T>>,
}
