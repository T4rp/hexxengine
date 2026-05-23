use std::borrow::Cow;

use glam::Vec2;

use crate::text::{FontHandle, FontManager};

pub enum HorizontalJustification {
    Left,
    Center,
    Right,
}

pub enum VerticalJustification {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphPositions {
    pub glyph: u64,
    pub offset: Vec2,
}

pub struct TextBox {
    pub horizontal_justification: HorizontalJustification,
    pub vertical_justification: VerticalJustification,
    pub font_height: u32,
    pub font: FontHandle,
    pub size: Vec2,
    pub text: Cow<'static, str>,

    pub glyph_positions: Vec<GlyphPositions>,
    pub layout_size: Vec2,

    pub is_dirty: bool,
}

impl TextBox {
    pub fn new(font: FontHandle) -> Self {
        Self {
            font,
            horizontal_justification: HorizontalJustification::Center,
            vertical_justification: VerticalJustification::Center,
            size: Vec2::ZERO,
            text: "".into(),
            is_dirty: true,
            glyph_positions: Vec::new(),
            layout_size: Vec2::ZERO,
            font_height: 16,
        }
    }

    pub fn from_text(font: FontHandle, font_height: u32, text: Cow<'static, str>) -> Self {
        Self {
            horizontal_justification: HorizontalJustification::Center,
            vertical_justification: VerticalJustification::Center,
            size: Vec2::ZERO,
            is_dirty: true,
            glyph_positions: Vec::new(),
            layout_size: Vec2::ZERO,
            font_height,
            text,
            font,
        }
    }

    pub fn calculate_layout(&mut self, font_manager: &mut FontManager) {
        if !self.is_dirty {
            return;
        }

        self.glyph_positions.clear();

        let mut pen_x = 0;
        let mut pen_y = 0;

        let mut width = 0;
        let mut max_ascent = 0;
        let mut max_descent = 0;

        for character in self.text.chars() {
            let glyph_data = font_manager
                .load_glyph(self.font, character as u64, self.font_height)
                .unwrap();

            width += glyph_data.metrics.hori_advance;

            let ascent = glyph_data.metrics.hori_bearing_y;
            let descent = glyph_data.metrics.height - ascent;

            if ascent > max_ascent {
                max_ascent = ascent
            }

            if descent > max_descent {
                max_descent = descent
            }

            self.glyph_positions.push(GlyphPositions {
                glyph: character as u64,
                offset: Vec2::new(pen_x as f32, pen_y as f32),
            });

            pen_x += glyph_data.advance.0;
            pen_y += glyph_data.advance.1;
        }

        self.layout_size = Vec2::new(
            width as f32 / 64.0,
            (max_ascent + max_descent) as f32 / 64.0,
        );

        self.is_dirty = false
    }

    pub fn set_text(&mut self, text: Cow<'static, str>) {
        self.text = text;
        self.is_dirty = true
    }

    pub fn set_size(&mut self, size: Vec2) {
        self.size = size;
        self.is_dirty = true
    }

    pub fn set_font(&mut self, font: FontHandle) {
        self.font = font;
        self.is_dirty = true;
    }

    pub fn set_font_height(&mut self, font_height: u32) {
        self.font_height = font_height;
        self.is_dirty = true;
    }
}
