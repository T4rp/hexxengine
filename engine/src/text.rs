use std::{collections::HashMap, fmt::Display, fs};

use image::{ImageBuffer, RgbaImage};
use paidtype::freetype::{
    FT_LOAD_DEFAULT, FT_Render_Mode__FT_RENDER_MODE_NORMAL, FT_Render_Mode__FT_RENDER_MODE_SDF,
};

use crate::{
    assets::ASSET_PATH,
    freetype::{Face, FreetypeError, FreetypeLibrary},
    shapes::Rect,
};

const MIN_BIN_LENGTH: u32 = 8;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct GlyphData {
    pub rect: Rect,
    pub advance: (i32, i32),
    pub is_empty: bool,
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

pub enum GlyphRenderMode {
    Normal,
    Sdf,
}

pub struct GlyphAtlas {
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub face: Face,
    pub bins: Vec<Rect>,
    pub glyphs: HashMap<GlyphKey, GlyphData>,
    pub render_mode: GlyphRenderMode,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum GlyphAtlasError {
    Freetype(FreetypeError),
    NoGlyphIndex(u64),
    NoBinFit,
}

impl Display for GlyphAtlasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlyphAtlasError::Freetype(freetype_error) => {
                write!(f, "freetype error: {}", freetype_error)
            }
            GlyphAtlasError::NoGlyphIndex(glyph) => {
                write!(f, "failed to load glyph '': {}", glyph)
            }
            GlyphAtlasError::NoBinFit => {
                write!(f, "couldnt fit glyph render into bitmap")
            }

            err => {
                write!(f, "{:?}", err)
            }
        }
    }
}

impl GlyphAtlas {
    pub fn new(render_mode: GlyphRenderMode, width: u32, height: u32) -> GlyphAtlas {
        let library = FreetypeLibrary::new().unwrap();
        let font_data = fs::read(format!("{}/unifont-17.0.03.otf", ASSET_PATH)).unwrap();
        let face = library.new_memory_face(&font_data, 0).unwrap();

        let pixel_width = width;
        let bitmap: Vec<u8> = vec![0; (pixel_width * height) as usize];

        let bins = vec![Rect {
            x: 0,
            y: 0,
            width,
            height,
        }];

        let glyphs = HashMap::new();

        Self {
            bitmap,
            width,
            height,
            face,
            bins,
            glyphs,
            render_mode,
        }
    }

    pub fn choose_bin(&self, width: u32, height: u32) -> Option<usize> {
        let mut chosen_bin = None;
        let mut chosen_bin_rating: Option<i32> = None;

        for i in 0..self.bins.len() {
            let bin = &self.bins[i];
            let rating_x: i32 = bin.width as i32 - width as i32;
            let rating_y = bin.height as i32 - height as i32;

            if rating_x < 0 || rating_y < 0 {
                continue;
            }

            let rating = rating_x + rating_y;

            if chosen_bin_rating.map_or(true, |f| f > rating) {
                chosen_bin_rating = Some(rating);
                chosen_bin = Some(i);
            }
        }

        chosen_bin
    }

    fn prune_bins(&mut self) {
        let bins_count = self.bins.len();
        let mut to_remove = vec![false; self.bins.len()];

        for i in 0..bins_count {
            if to_remove[i] {
                continue;
            }

            let i_bin = &self.bins[i];

            for j in 0..bins_count {
                if i == j {
                    continue;
                }

                if to_remove[j] {
                    continue;
                }

                let j_bin = &self.bins[j];

                if i_bin.contains(j_bin) {
                    to_remove[j] = true;
                }
            }
        }

        let mut to_remove_iter = to_remove.into_iter();
        self.bins.retain(|_| !to_remove_iter.next().unwrap());
    }

    fn break_bin(&mut self, bin_index: usize, rect: Rect) {
        // TODO: fix bug where bins are unoptimally split

        let bin = &self.bins[bin_index];

        let bin_right = Rect {
            x: rect.x + rect.width,
            y: bin.y,
            width: (bin.x + bin.width).saturating_sub(rect.x + rect.width),
            height: bin.height,
        };

        let bin_down = Rect {
            x: bin.x,
            y: rect.y + rect.height,
            width: bin.width,
            height: (bin.y + bin.height).saturating_sub(rect.y + rect.height),
        };

        self.bins.swap_remove(bin_index);

        let initial_len = self.bins.len();

        if bin_right.width > MIN_BIN_LENGTH && bin_right.height > MIN_BIN_LENGTH {
            self.bins.push(bin_right);
        }

        if bin_down.width > MIN_BIN_LENGTH && bin_down.height > MIN_BIN_LENGTH {
            self.bins.push(bin_down);
        }

        for i in 0..initial_len {
            let bin = &self.bins[i];
            if !bin.intersects(&rect) {
                self.bins.push(bin.clone());
                continue;
            };

            let left = Rect {
                x: bin.x,
                y: bin.y,
                width: rect.x.saturating_sub(bin.x),
                height: bin.height,
            };

            let up = Rect {
                x: bin.x,
                y: bin.y,
                width: bin.width,
                height: rect.y.saturating_sub(bin.y),
            };

            let right = Rect {
                x: rect.x + rect.width,
                y: bin.y,
                width: (bin.x + bin.width).saturating_sub(rect.x + rect.width),
                height: bin.height,
            };

            let down = Rect {
                x: bin.x,
                y: rect.y + rect.height,
                width: bin.width,
                height: (bin.y + bin.height).saturating_sub(rect.y + rect.height),
            };

            for rect in [right, down, left, up] {
                if rect.width > MIN_BIN_LENGTH && rect.height > MIN_BIN_LENGTH {
                    self.bins.push(rect);
                }
            }
        }

        let new_len = self.bins.len();

        let new_size = new_len - initial_len;

        for i in 0..new_size {
            self.bins[i] = self.bins[initial_len + i];
        }

        self.bins.truncate(new_size);

        self.prune_bins();
    }

    pub fn load_glyph(&mut self, glyph: u64, font_heigth: u32) -> Result<(), GlyphAtlasError> {
        let glyph_key = GlyphKey::new(glyph, font_heigth);

        if self.glyphs.contains_key(&glyph_key) {
            return Ok(());
        }

        let glyph_index = self
            .face
            .get_char_index(glyph_key.glyph)
            .ok_or(GlyphAtlasError::NoGlyphIndex(glyph_key.glyph))?;

        self.face
            .set_pixel_sizes(0, glyph_key.font_height)
            .map_err(|err| GlyphAtlasError::Freetype(err))?;

        self.face
            .load_glyph(glyph_index, FT_LOAD_DEFAULT)
            .map_err(|err| GlyphAtlasError::Freetype(err))?;

        match self.render_mode {
            GlyphRenderMode::Normal => self
                .face
                .render_glyph(FT_Render_Mode__FT_RENDER_MODE_NORMAL),
            GlyphRenderMode::Sdf => self.face.render_glyph(FT_Render_Mode__FT_RENDER_MODE_SDF),
        }
        .map_err(|err| GlyphAtlasError::Freetype(err))?;

        let (advance_x, advance_y) = self.face.get_glyph_advance();

        let Some(bitmap_data) = self.face.get_bitmap_data() else {
            let glyph = GlyphData {
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                },
                advance: (advance_x, advance_y),
                is_empty: true,
            };

            self.glyphs.insert(glyph_key, glyph);

            return Ok(());
        };

        let chosen_bin_index = self
            .choose_bin(bitmap_data.width, bitmap_data.rows)
            .ok_or(GlyphAtlasError::NoBinFit)?;

        let chosen_bin = self.bins[chosen_bin_index];

        let glyph_bounds = Rect {
            x: chosen_bin.x,
            y: chosen_bin.y,
            width: bitmap_data.width,
            height: bitmap_data.rows,
        };

        for y in 0..glyph_bounds.height {
            for x in 0..glyph_bounds.width {
                let buffer_offset = (x + y * glyph_bounds.width) as usize;
                let bitmap_offset =
                    (glyph_bounds.x + x + (glyph_bounds.y + y) * self.width) as usize;

                self.bitmap[bitmap_offset] = bitmap_data.buffer[buffer_offset]
            }
        }

        let glyph = GlyphData {
            rect: glyph_bounds,
            advance: (advance_x, advance_y),
            is_empty: false,
        };

        self.glyphs.insert(glyph_key, glyph);
        self.break_bin(chosen_bin_index, glyph_bounds);

        Ok(())
    }

    pub fn debug_render(&self) -> RgbaImage {
        let mut bitmap_rgba = vec![0; (self.width * self.height * 4) as usize];

        for (i, col) in self.bitmap.iter().enumerate() {
            let r = i * 4;
            bitmap_rgba[r] = *col;
            bitmap_rgba[r + 1] = *col;
            bitmap_rgba[r + 2] = *col;
            bitmap_rgba[r + 3] = 255;
        }

        for bin in self.bins.iter() {
            for i in 0..bin.width {
                let top = (bin.x + i + bin.y * self.width) as usize * 4;
                let bottom = (bin.x + i + (bin.y + bin.height - 1) * self.width) as usize * 4;

                bitmap_rgba[top + 1] = 255;
                bitmap_rgba[bottom + 1] = 255;
            }

            for i in 0..bin.height {
                let left = (bin.x + (bin.y + i) * self.width) as usize * 4;
                let right = ((bin.x + bin.width - 1) + (bin.y + i) * self.width) as usize * 4;

                bitmap_rgba[left + 1] = 255;
                bitmap_rgba[right + 1] = 255;
            }
        }

        let img_buff: RgbaImage =
            ImageBuffer::from_raw(self.width, self.height, bitmap_rgba).unwrap();

        img_buff
    }

    pub fn get_glyph(&self, glyph: u64, font_height: u32) -> Option<&GlyphData> {
        let glyph_key = GlyphKey::new(glyph, font_height);
        self.glyphs.get(&glyph_key)
    }

    pub fn get_glyphs(&self, text: &str, font_height: u32) -> Vec<&GlyphData> {
        let mut glyphs = Vec::with_capacity(text.chars().count());

        for character in text.chars() {
            let glyph_data = self
                .get_glyph(character as u64, font_height)
                .unwrap_or(&GlyphData {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                    },
                    advance: (0, 0),
                    is_empty: true,
                });

            glyphs.push(glyph_data)
        }

        glyphs
    }

    pub fn has_glyph(&self, glyph: u64, font_height: u32) -> bool {
        let glyph_key = GlyphKey::new(glyph, font_height);
        self.glyphs.contains_key(&glyph_key)
    }

    pub fn load_glyphs(&mut self, text: &str, font_height: u32) -> Result<(), GlyphAtlasError> {
        for char in text.chars() {
            self.load_glyph(char as u64, font_height)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::text::{GlyphAtlas, GlyphRenderMode};

    #[test]
    fn creation() {
        GlyphAtlas::new(GlyphRenderMode::Normal, 256, 256);
    }

    #[test]
    fn load_glyph_normal() {
        let mut atlas = GlyphAtlas::new(GlyphRenderMode::Normal, 256, 256);

        for height in [32, 24, 18, 16, 12] {
            for i in 32..128 {
                atlas.load_glyph(i as u64, height).unwrap();
            }
        }

        atlas.debug_render().save("glyph_atlas_test.png").unwrap();
    }

    #[test]
    fn load_glyph_sdf() {
        let mut atlas = GlyphAtlas::new(GlyphRenderMode::Sdf, 512, 512);

        for i in 32..128 {
            atlas.load_glyph(i as u64, 48).unwrap();
        }

        atlas
            .debug_render()
            .save("sdf_glyph_atlas_test.png")
            .unwrap();
    }

    #[test]
    fn get_glyphs() {
        let mut atlas = GlyphAtlas::new(GlyphRenderMode::Normal, 128, 128);

        for height in [18] {
            for i in 32..128 {
                atlas.load_glyph(i as u64, height).unwrap();
            }
        }

        atlas.get_glyphs("the quick brown fox jumps over the lazy dog", 18);
    }
}
