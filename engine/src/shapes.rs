use glam::UVec2;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn contains(&self, rect: &Rect) -> bool {
        rect.x >= self.x
            && rect.x + rect.width <= self.x + self.width
            && rect.y >= self.y
            && rect.y + rect.height <= self.y + self.height
    }

    pub fn intersects(&self, rect: &Rect) -> bool {
        self.x + self.width >= rect.x
            && rect.x + rect.width >= self.x
            && self.y + self.height >= rect.y
            && rect.y + rect.height >= self.y
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct Region2d {
    pub top_left: UVec2,
    pub bottom_right: UVec2,
}
