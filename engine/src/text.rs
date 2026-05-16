use std::{collections::HashMap, fmt::Display, fs};

use glam::UVec2;
use image::{ImageBuffer, RgbaImage};
use paidtype::freetype::{
    FT_LOAD_DEFAULT, FT_Render_Mode__FT_RENDER_MODE_NORMAL, FT_Render_Mode__FT_RENDER_MODE_SDF,
};

use crate::{
    assets::ASSET_PATH,
    font_manager::{FontHandle, FontManager, FontManagerError, GlyphRenderMode},
    freetype::{Face, FreetypeError, FreetypeLibrary},
    shapes::{Boundsi64, Rect, Region2d},
};

const MIN_BIN_LENGTH: u32 = 8;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct GlyphAtlasRect {
    pub rect: Rect,
    pub is_empty: bool,
}

#[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct GlyphAtlasKey {
    font_handle: FontHandle,
    glyph: u64,
    font_height: u32,
}

impl GlyphAtlasKey {
    fn new(font_handle: FontHandle, glyph: u64, font_height: u32) -> Self {
        Self {
            font_handle,
            glyph,
            font_height,
        }
    }
}

pub struct GlyphAtlas {
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub bins: Vec<Rect>,
    pub glyphs: HashMap<GlyphAtlasKey, GlyphAtlasRect>,
    pub render_mode: GlyphRenderMode,
    pub dirty_region: Option<Region2d>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum GlyphAtlasError {
    FontManager(FontManagerError),
    NoGlyphIndex(u64),
    NoBinFit,
}

impl From<FontManagerError> for GlyphAtlasError {
    fn from(value: FontManagerError) -> Self {
        GlyphAtlasError::FontManager(value)
    }
}

impl GlyphAtlas {
    pub fn new(render_mode: GlyphRenderMode, width: u32, height: u32) -> GlyphAtlas {
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
            bins,
            glyphs,
            render_mode,
            dirty_region: None,
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

    pub fn render_glyph(
        &mut self,
        font_manager: &mut FontManager,
        font_handle: FontHandle,
        glyph: u64,
        font_height: u32,
    ) -> Result<(), GlyphAtlasError> {
        let glyph_key = GlyphAtlasKey::new(font_handle, glyph, font_height);

        if self.glyphs.contains_key(&glyph_key) {
            return Ok(());
        }

        font_manager
            .load_glyph(font_handle, glyph, font_height)
            .unwrap();

        let bitmap_data = font_manager
            .render_glyph(self.render_mode, font_handle, glyph, font_height)
            .unwrap();

        let Some(bitmap_data) = bitmap_data else {
            let glyph = GlyphAtlasRect {
                rect: Rect {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                },
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

        let glyph = GlyphAtlasRect {
            rect: glyph_bounds,
            is_empty: false,
        };

        self.update_dirty_region(&glyph_bounds);

        self.glyphs.insert(glyph_key, glyph);
        self.break_bin(chosen_bin_index, glyph_bounds);

        Ok(())
    }

    pub fn flush_dirty_region(&mut self) {
        self.dirty_region = None;
    }

    fn update_dirty_region(&mut self, rect: &Rect) {
        if let Some(region) = self.dirty_region.as_mut() {
            region.top_left = region.top_left.min(UVec2 {
                x: rect.x,
                y: rect.y,
            });
            region.bottom_right = region.bottom_right.max(UVec2 {
                x: rect.x + rect.width,
                y: rect.y + rect.height,
            });
        } else {
            self.dirty_region = Some(Region2d {
                top_left: UVec2::new(rect.x, rect.y),
                bottom_right: UVec2::new(rect.x + rect.width, rect.y + rect.height),
            })
        };
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

    pub fn get_glyph(
        &self,
        font_handle: FontHandle,
        glyph: u64,
        font_height: u32,
    ) -> Option<&GlyphAtlasRect> {
        let glyph_key = GlyphAtlasKey::new(font_handle, glyph, font_height);
        self.glyphs.get(&glyph_key)
    }

    pub fn get_glyphs(
        &self,
        font_handle: FontHandle,
        text: &str,
        font_height: u32,
    ) -> Vec<&GlyphAtlasRect> {
        let mut rects = Vec::new();

        for char in text.chars() {
            let glyph_key = GlyphAtlasKey::new(font_handle, char as u64, font_height);

            match self.glyphs.get(&glyph_key) {
                Some(rect) => rects.push(rect),
                None => rects.push(&GlyphAtlasRect {
                    rect: Rect {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                    },
                    is_empty: true,
                }),
            }
        }

        rects
    }

    pub fn load_glyphs(
        &mut self,
        font_manager: &mut FontManager,
        font_handle: FontHandle,
        text: &str,
        font_height: u32,
    ) {
        for char in text.chars() {
            self.render_glyph(font_manager, font_handle, char as u64, font_height)
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        font_manager::{self, FontManager, FontManagerError},
        text::{ASSET_PATH, GlyphAtlas, GlyphRenderMode},
    };

    fn get_unifont_path() -> String {
        format!("{}/unifont-17.0.03.otf", ASSET_PATH)
    }

    #[test]
    fn creation() {
        GlyphAtlas::new(GlyphRenderMode::Normal, 256, 256);
    }

    #[test]
    fn load_glyph_normal() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(get_unifont_path()).unwrap();
        let font = font_manager.load_font(&font_data).unwrap();

        let mut atlas = GlyphAtlas::new(GlyphRenderMode::Normal, 256, 256);

        for height in [32, 24, 18, 16, 12] {
            for i in 32..128 {
                atlas
                    .render_glyph(&mut font_manager, font, i, height)
                    .unwrap()
            }
        }

        atlas.debug_render().save("glyph_atlas_test.png").unwrap();
    }

    #[test]
    fn load_glyph_sdf() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(get_unifont_path()).unwrap();
        let font = font_manager.load_font(&font_data).unwrap();

        let mut atlas = GlyphAtlas::new(GlyphRenderMode::Sdf, 512, 512);

        for i in 32..128 {
            atlas.render_glyph(&mut font_manager, font, i, 48).unwrap()
        }

        atlas
            .debug_render()
            .save("sdf_glyph_atlas_test.png")
            .unwrap();
    }

    #[test]
    fn get_glyphs() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(get_unifont_path()).unwrap();
        let font = font_manager.load_font(&font_data).unwrap();

        let mut atlas = GlyphAtlas::new(GlyphRenderMode::Normal, 128, 128);

        for height in [18] {
            for i in 32..128 {
                atlas
                    .render_glyph(&mut font_manager, font, i as u64, height)
                    .unwrap();
            }
        }

        atlas.get_glyphs(font, "the quick brown fox jumps over the lazy dog", 18);
    }

    #[test]
    fn dirty_region() {
        let mut font_manager = FontManager::new();
        let font_data = fs::read(get_unifont_path()).unwrap();
        let font = font_manager.load_font(&font_data).unwrap();

        let mut atlas = GlyphAtlas::new(GlyphRenderMode::Normal, 256, 256);

        atlas
            .render_glyph(&mut font_manager, font, 67 as u64, 18)
            .unwrap();
        assert_eq!(atlas.dirty_region.is_some(), true);

        atlas.flush_dirty_region();
        assert_eq!(atlas.dirty_region, None);

        atlas
            .render_glyph(&mut font_manager, font, 67 as u64, 18)
            .unwrap();
        assert_eq!(atlas.dirty_region, None);

        atlas
            .render_glyph(&mut font_manager, font, 67 as u64, 24)
            .unwrap();
        assert_eq!(atlas.dirty_region.is_some(), true);
    }
}
