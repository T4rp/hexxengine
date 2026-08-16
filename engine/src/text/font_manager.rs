use std::collections::HashMap;

use glam::I64Vec2;
use paidtype::freetype::{
    FT_LOAD_DEFAULT, FT_Render_Mode__FT_RENDER_MODE_NORMAL, FT_Render_Mode__FT_RENDER_MODE_SDF,
};
use thunderdome::{Arena, Index};

use crate::{
    shapes::Boundsi64,
    text::freetype::{Face, FreetypeError, FreetypeLibrary, GlyphBitmap, GlyphMetrics},
};

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Eq, Debug, Ord)]
pub struct FontHandle(pub(crate) Index);

#[derive(Debug, Clone, Copy)]
pub struct GlyphData {
    pub advance: (i32, i32),
    pub bitmap_top: i32,
    pub bitmap_left: i32,
    pub glyph_index: u32,
    pub metrics: GlyphMetrics,
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

pub struct Font {
    face: Face,
    glyph_cache: HashMap<GlyphKey, GlyphData>,
}

impl Font {
    pub fn get_glyph(&self, glyph: u64, font_height: u32) -> Option<GlyphData> {
        let glyph_key = GlyphKey::new(glyph, font_height);
        self.glyph_cache.get(&glyph_key).copied()
    }

    pub fn set_font_height(&mut self, font_height: u32) -> Result<(), FontManagerError> {
        self.face.set_pixel_sizes(0, font_height)?;
        Ok(())
    }

    pub fn get_char_index(&self, character: u64) -> Option<u32> {
        self.face.get_char_index(character)
    }

    pub fn get_kerning(
        &mut self,
        left_glyph: u32,
        right_glyph: u32,
    ) -> Result<I64Vec2, FontManagerError> {
        Ok(self.face.get_glyph_kerning(left_glyph, right_glyph)?)
    }

    pub fn render_glyph(
        &mut self,
        render_mode: GlyphRenderMode,
        glyph: u64,
        font_height: u32,
    ) -> Result<Option<GlyphBitmap<'_>>, FontManagerError> {
        let glyph_index = self
            .face
            .get_char_index(glyph)
            .ok_or(FontManagerError::NoGlyphIndex(glyph))?;

        self.face.set_pixel_sizes(0, font_height)?;
        self.face.load_glyph(glyph_index, FT_LOAD_DEFAULT)?;

        match render_mode {
            GlyphRenderMode::Normal => self
                .face
                .render_glyph(FT_Render_Mode__FT_RENDER_MODE_NORMAL),
            GlyphRenderMode::Sdf => self.face.render_glyph(FT_Render_Mode__FT_RENDER_MODE_SDF),
        }?;

        Ok(self.face.get_bitmap_data())
    }

    pub fn load_glyph(
        &mut self,
        glyph: u64,
        font_height: u32,
    ) -> Result<GlyphData, FontManagerError> {
        let glyph_key = GlyphKey::new(glyph, font_height);

        if let Some(glyph_data) = self.glyph_cache.get(&glyph_key) {
            return Ok(*glyph_data);
        }

        let glyph_index = self
            .face
            .get_char_index(glyph)
            .ok_or(FontManagerError::NoGlyphIndex(glyph))?;

        self.face.set_pixel_sizes(0, font_height)?;
        self.face.load_glyph(glyph_index, FT_LOAD_DEFAULT)?;

        let cbox = self.face.get_glyph_cbox()?;
        let (advance_x, advance_y) = self.face.get_glyph_advance();
        let (bitmap_left, bitmap_top) = self.face.get_glyph_left_top();
        let metrics = self.face.get_glyph_metrics();

        let glyph_data = GlyphData {
            advance: (advance_x, advance_y),
            bitmap_top,
            bitmap_left,
            glyph_index,
            cbox,
            metrics,
        };

        self.glyph_cache.insert(glyph_key, glyph_data);

        Ok(glyph_data)
    }
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
    fonts: Arena<Font>,
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FontManager {
    pub fn new() -> Self {
        let freetype = FreetypeLibrary::new().unwrap();
        let fonts = Arena::new();

        Self { freetype, fonts }
    }

    pub fn load_font_with_index(
        &mut self,
        index: Index,
        font_data: &[u8],
    ) -> Result<FontHandle, FreetypeError> {
        let face = self.freetype.new_memory_face(font_data, 0)?;

        let face_data = Font {
            face,
            glyph_cache: HashMap::new(),
        };

        self.fonts.insert_at(index, face_data);
        Ok(FontHandle(index))
    }

    pub fn load_font(&mut self, font_data: &[u8]) -> Result<FontHandle, FreetypeError> {
        let face = self.freetype.new_memory_face(font_data, 0)?;

        let face_data = Font {
            face,
            glyph_cache: HashMap::new(),
        };

        let index = self.fonts.insert(face_data);
        Ok(FontHandle(index))
    }

    pub fn get_font(&mut self, font_handle: FontHandle) -> &mut Font {
        self.fonts.get_mut(font_handle.0).unwrap()
    }

    pub fn get_font_ref(&self, font_handle: FontHandle) -> &Font {
        self.fonts.get(font_handle.0).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{assets::ASSET_PATH, text::FontManager};

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
        let font_handle = font_manager.load_font(&font_data).unwrap();

        let font = font_manager.get_font(font_handle);

        for height in [32, 24, 18, 16, 12] {
            for i in 32..128 {
                font.load_glyph(i as u64, height).unwrap();
            }
        }
    }
}
