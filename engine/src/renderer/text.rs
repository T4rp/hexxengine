use std::fs;

use crate::{
    assets::ASSET_PATH,
    renderer::freetype::{self, Face, FreetypeLibrary},
};

pub struct GlyphAtlas {
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub face: Face,
}

impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> GlyphAtlas {
        let library = FreetypeLibrary::new().unwrap();
        let font_data = fs::read(format!("{}/unifont-17.0.03.otf", ASSET_PATH)).unwrap();
        let face = library.new_memory_face(&font_data, 0).unwrap();

        let bitmap: Vec<u8> = Vec::with_capacity(width as usize * height as usize);

        Self {
            bitmap,
            width,
            height,
            face,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::renderer::{freetype::FreetypeLibrary, text::GlyphAtlas};

    #[test]
    fn creation() {
        GlyphAtlas::new(256, 256);
    }
}
