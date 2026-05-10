use thunderdome::{Arena, Index};

use crate::freetype::{Face, FreetypeError, FreetypeLibrary};

pub struct FontHandle(Index);

pub struct FontManager {
    freetype: FreetypeLibrary,
    fonts: Arena<Face>,
}

impl FontManager {
    pub fn new() -> Self {
        let freetype = FreetypeLibrary::new().unwrap();
        let fonts = Arena::new();

        Self { freetype, fonts }
    }

    pub fn load_font(&mut self, font_data: &[u8]) -> Result<FontHandle, FreetypeError> {
        let face = self.freetype.new_memory_face(&font_data, 0)?;
        let index = self.fonts.insert(face);
        Ok(FontHandle(index))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{assets::ASSET_PATH, font_manager::FontManager};

    #[test]
    fn init() {
        let _font_manager = FontManager::new();
    }

    #[test]
    fn load_font() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(&format!("{}/unifont-17.0.03.otf", ASSET_PATH)).unwrap();
        font_manager.load_font(&font_data).unwrap();
    }
}
