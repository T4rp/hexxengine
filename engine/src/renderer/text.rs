use std::{collections::HashMap, fs};

use paidtype::freetype::{
    FT_LOAD_DEFAULT, FT_Render_Mode__FT_RENDER_MODE_LCD, FT_Render_Mode__FT_RENDER_MODE_NORMAL,
};

use crate::{
    assets::ASSET_PATH,
    freetype::{Face, FreetypeLibrary},
};

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

pub struct GlyphAtlas {
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub face: Face,
    pub bins: Vec<Rect>,
    pub glyphs: HashMap<GlyphKey, GlyphBounds>,
}

impl GlyphAtlas {
    pub fn new(width: u32, height: u32) -> GlyphAtlas {
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
        }
    }

    pub fn choose_bin(&self, width: u32, height: u32) -> Option<usize> {
        self.bins.iter().enumerate().find_map(|(i, r)| {
            let fits = r.width >= width && r.height >= height;
            match fits {
                true => Some(i),
                false => None,
            }
        })
    }

    fn prune_bins(&mut self) {
        let bins_count = self.bins.len();
        let mut to_remove = vec![false; self.bins.len()];

        for i in 0..bins_count {
            if to_remove[i] {
                continue;
            }

            for j in 0..bins_count {
                if i == j {
                    continue;
                }

                if to_remove[j] {
                    continue;
                }

                if self.bins[i].contains(&self.bins[j]) {
                    to_remove[j] = true;
                }
            }
        }

        let mut to_remove_iter = to_remove.into_iter();
        self.bins.retain(|_| !to_remove_iter.next().unwrap());
    }

    fn break_bin(&mut self, bin_index: usize, rect: Rect) {
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
        self.bins.push(bin_right);
        self.bins.push(bin_down);

        // TODO: break up all other rects

        self.prune_bins();
    }

    fn insert_glyph(&mut self, glyph_key: GlyphKey) -> Option<&GlyphBounds> {
        let Some(glyph_index) = self.face.get_char_index(glyph_key.glyph) else {
            return None;
        };

        self.face.set_pixel_sizes(0, glyph_key.font_height).unwrap();

        self.face.load_glyph(glyph_index, FT_LOAD_DEFAULT).unwrap();
        self.face
            .render_glyph(FT_Render_Mode__FT_RENDER_MODE_LCD)
            .unwrap();

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

    pub fn load_glyph(&mut self, glyph: u64) -> Option<&GlyphBounds> {
        let key = GlyphKey::new(glyph, 16);

        if self.glyphs.contains_key(&key) {
            return self.glyphs.get(&key);
        }

        self.insert_glyph(key)
    }
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, RgbaImage};

    use crate::renderer::text::GlyphAtlas;

    #[test]
    fn creation() {
        GlyphAtlas::new(256, 256);
    }

    #[test]
    fn load_glyph() {
        let mut atlas = GlyphAtlas::new(256, 256);

        for i in 65..123 {
            atlas.load_glyph(i as u64);
        }

        let mut bitmap_rgba = vec![0; (atlas.width * atlas.height * 4) as usize];

        for (i, col) in atlas.bitmap.iter().enumerate() {
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
