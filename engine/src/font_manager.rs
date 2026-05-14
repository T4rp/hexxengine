use std::collections::HashMap;

use paidtype::freetype::{
    FT_LOAD_DEFAULT, FT_Render_Mode__FT_RENDER_MODE_NORMAL, FT_Render_Mode__FT_RENDER_MODE_SDF,
};
use thunderdome::{Arena, Index};

use crate::{
    freetype::{Face, FreetypeError, FreetypeLibrary, GlyphBitmap},
    shapes::{Boundsi64, Rect},
};

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Debug, Ord)]
pub struct FontHandle(Index);

#[derive(Debug, Clone, Copy)]
pub struct GlyphData {
    pub advance: (i32, i32),
    pub bitmap_top: i32,
    pub bitmap_left: i32,
    pub glyph_index: u32,
    pub cbox: Boundsi64,
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct GlyphKey {
    glyph: u64,
    font_height: u32,
}

impl GlyphKey {
    fn new(glyph: u64, font_height: u32) -> Self {
        Self { glyph, font_height }
    }
}

#[derive(Clone, Copy)]
pub enum GlyphRenderMode {
    Normal,
    Sdf,
}

pub struct FontData {
    face: Face,
    glyph_cache: HashMap<GlyphKey, GlyphData>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum FontManagerError {
    Freetype(FreetypeError),
    NoGlyphIndex(u64),
}

impl From<FreetypeError> for FontManagerError {
    fn from(value: FreetypeError) -> Self {
        FontManagerError::Freetype(value)
    }
}

pub struct FontManager {
    freetype: FreetypeLibrary,
    fonts: Arena<FontData>,
}

impl FontManager {
    pub fn new() -> Self {
        let freetype = FreetypeLibrary::new().unwrap();
        let fonts = Arena::new();

        Self { freetype, fonts }
    }

    pub fn load_font(&mut self, font_data: &[u8]) -> Result<FontHandle, FreetypeError> {
        let face = self.freetype.new_memory_face(&font_data, 0)?;

        let face_data = FontData {
            face,
            glyph_cache: HashMap::new(),
        };

        let index = self.fonts.insert(face_data);
        Ok(FontHandle(index))
    }

    pub fn render_glyph(
        &mut self,
        render_mode: GlyphRenderMode,
        font_handle: FontHandle,
        glyph: u64,
        font_height: u32,
    ) -> Result<Option<GlyphBitmap>, FontManagerError> {
        let font = self.fonts.get_mut(font_handle.0).unwrap();

        let glyph_index = font
            .face
            .get_char_index(glyph)
            .ok_or(FontManagerError::NoGlyphIndex(glyph))?;

        font.face.set_pixel_sizes(0, font_height)?;
        font.face.load_glyph(glyph_index, FT_LOAD_DEFAULT)?;

        match render_mode {
            GlyphRenderMode::Normal => font
                .face
                .render_glyph(FT_Render_Mode__FT_RENDER_MODE_NORMAL),
            GlyphRenderMode::Sdf => font.face.render_glyph(FT_Render_Mode__FT_RENDER_MODE_SDF),
        }?;

        Ok(font.face.get_bitmap_data())
    }

    pub fn get_glyph(
        &self,
        font_handle: FontHandle,
        glyph: u64,
        font_height: u32,
    ) -> Option<GlyphData> {
        let glyph_key = GlyphKey::new(glyph, font_height);
        let font = self.fonts.get(font_handle.0).unwrap();
        font.glyph_cache.get(&glyph_key).map(|data| *data)
    }

    pub fn load_glyph(
        &mut self,
        font_handle: FontHandle,
        glyph: u64,
        font_height: u32,
    ) -> Result<GlyphData, FontManagerError> {
        let glyph_key = GlyphKey::new(glyph, font_height);

        let font = self.fonts.get_mut(font_handle.0).unwrap();

        if let Some(glyph_data) = font.glyph_cache.get(&glyph_key) {
            return Ok(*glyph_data);
        }

        let glyph_index = font
            .face
            .get_char_index(glyph)
            .ok_or(FontManagerError::NoGlyphIndex(glyph))?;

        font.face.set_pixel_sizes(0, font_height)?;
        font.face.load_glyph(glyph_index, FT_LOAD_DEFAULT)?;

        let cbox = font.face.get_glyph_cbox()?;
        let (advance_x, advance_y) = font.face.get_glyph_advance();
        let (bitmap_left, bitmap_top) = font.face.get_glyph_left_top();

        let glyph_data = GlyphData {
            advance: (advance_x, advance_y),
            bitmap_top,
            bitmap_left,
            glyph_index,
            cbox,
        };

        font.glyph_cache.insert(glyph_key, glyph_data);

        Ok(glyph_data)
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

    fn load_glyphs() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(&format!("{}/unifont-17.0.03.otf", ASSET_PATH)).unwrap();
        let font = font_manager.load_font(&font_data).unwrap();

        for height in [32, 24, 18, 16, 12] {
            for i in 32..128 {
                font_manager.load_glyph(font, i as u64, height).unwrap();
            }
        }
    }
}
