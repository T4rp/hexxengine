use std::{collections::HashMap, fs};

use paidtype::freetype::{
    FT_LOAD_DEFAULT, FT_Render_Mode__FT_RENDER_MODE_LCD, FT_Render_Mode__FT_RENDER_MODE_NORMAL,
};

use crate::{
    assets::ASSET_PATH,
    freetype::{Face, FreetypeLibrary},
};

const MIN_BIN_LENGTH: u32 = 8;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn contains(&self, rect: &Rect) -> bool {
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct GlyphBounds {
    rect: Rect,
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
    Lcd,
}

pub struct GlyphAtlas {
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub face: Face,
    pub bins: Vec<Rect>,
    pub glyphs: HashMap<GlyphKey, GlyphBounds>,
    pub render_mode: GlyphRenderMode,
}

impl GlyphAtlas {
    pub fn new(render_mode: GlyphRenderMode, width: u32, height: u32) -> GlyphAtlas {
        let library = FreetypeLibrary::new().unwrap();
        let font_data = fs::read(format!("{}/unifont-17.0.03.otf", ASSET_PATH)).unwrap();
        let face = library.new_memory_face(&font_data, 0).unwrap();

        let bitmap: Vec<u8> = vec![0; (width * height) as usize];

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
            x: bin.x + rect.width,
            y: bin.y,
            width: bin.width - rect.width,
            height: bin.height,
        };

        let bin_down = Rect {
            x: bin.x,
            y: bin.y + rect.height,
            width: bin.width,
            height: bin.height - rect.height,
        };

        self.bins.swap_remove(bin_index);

        let initial_len = self.bins.len();

        if bin_right.width >= MIN_BIN_LENGTH && bin_right.height >= MIN_BIN_LENGTH {
            self.bins.push(bin_right);
        }

        if bin_down.width >= MIN_BIN_LENGTH && bin_down.height >= MIN_BIN_LENGTH {
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
                width: bin
                    .width
                    .saturating_sub(rect.x)
                    .saturating_sub(bin.x)
                    .saturating_sub(rect.width),
                height: bin.height,
            };

            let down = Rect {
                x: bin.x,
                y: rect.y + rect.height,
                width: bin.width,
                height: bin
                    .height
                    .saturating_sub(rect.y)
                    .saturating_sub(bin.y)
                    .saturating_sub(rect.height),
            };

            for rect in [right, down, left, up] {
                if rect.width >= MIN_BIN_LENGTH && rect.height >= MIN_BIN_LENGTH {
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

    fn insert_glyph(&mut self, glyph_key: GlyphKey) -> Option<&GlyphBounds> {
        let Some(glyph_index) = self.face.get_char_index(glyph_key.glyph) else {
            return None;
        };

        self.face.set_pixel_sizes(0, glyph_key.font_height).unwrap();

        self.face.load_glyph(glyph_index, FT_LOAD_DEFAULT).unwrap();

        let ft_render_mode = match self.render_mode {
            GlyphRenderMode::Normal => FT_Render_Mode__FT_RENDER_MODE_NORMAL,
            GlyphRenderMode::Lcd => FT_Render_Mode__FT_RENDER_MODE_LCD,
        };

        self.face.render_glyph(ft_render_mode).unwrap();

        let bitmap_data = self.face.get_bitmap_data();

        let chosen_bin_index = self
            .choose_bin(bitmap_data.width, bitmap_data.rows)
            .expect("couldnt find suitable bin");

        let chosen_bin = self.bins[chosen_bin_index];

        let glyph_bounds = Rect {
            x: chosen_bin.x,
            y: chosen_bin.y,
            width: bitmap_data.width,
            height: bitmap_data.rows,
        };

        println!("{:#?}", glyph_bounds);

        for y in 0..glyph_bounds.height {
            for x in 0..glyph_bounds.width {
                let buffer_offset = x + y * glyph_bounds.width;
                let bitmap_offset = glyph_bounds.x + x + (glyph_bounds.y + y) * self.width;
                println!("{}", buffer_offset);
                println!("{}", buffer_offset);
                self.bitmap[bitmap_offset as usize] = bitmap_data.buffer[buffer_offset as usize]
            }
        }

        let glyph = GlyphBounds { rect: glyph_bounds };

        self.glyphs.insert(glyph_key, glyph);
        self.break_bin(chosen_bin_index, glyph_bounds);

        self.glyphs.get(&glyph_key)
    }

    pub fn load_glyph(&mut self, glyph: u64, font_heigth: u32) -> Option<&GlyphBounds> {
        let key = GlyphKey::new(glyph, font_heigth);

        if self.glyphs.contains_key(&key) {
            return self.glyphs.get(&key);
        }

        self.insert_glyph(key)
    }
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, RgbaImage};

    use crate::renderer::text::{GlyphAtlas, GlyphRenderMode};

    #[test]
    fn creation() {
        GlyphAtlas::new(GlyphRenderMode::Normal, 256, 256);
    }

    #[test]
    fn load_glyph() {
        let mut atlas = GlyphAtlas::new(GlyphRenderMode::Normal, 512, 512);

        for i in 65..123 {
            atlas.load_glyph(i as u64, 32);
        }

        for i in 65..123 {
            atlas.load_glyph(i as u64, 16);
        }

        for i in 65..123 {
            atlas.load_glyph(i as u64, 8);
        }

        let mut bitmap_rgba = vec![0; (atlas.width * atlas.height * 4) as usize];

        let marked_bitmap = atlas.bitmap;

        for (i, col) in marked_bitmap.iter().enumerate() {
            let r = i * 4;
            bitmap_rgba[r] = *col;
            bitmap_rgba[r + 1] = *col;
            bitmap_rgba[r + 2] = *col;
            bitmap_rgba[r + 3] = 255;
        }

        let img_buff: RgbaImage =
            ImageBuffer::from_raw(atlas.width, atlas.height, bitmap_rgba).unwrap();

        img_buff.save("glyph_atlas_test.png").unwrap();
    }
}
