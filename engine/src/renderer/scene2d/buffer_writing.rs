use ash::vk::{self, Rect2D};
use glam::{Vec2, Vec4, vec2};

use crate::{
    renderer::scene2d::Vertex2d,
    scene::{UiFrame, UiText},
    shapes::{Boundsi32, Rect},
    text::GlyphAtlas,
};

pub fn push_frame_verts(
    window_extent: vk::Extent2D,
    ui_frame: &UiFrame,
    vertices: &mut Vec<Vertex2d>,
    indices: &mut Vec<u16>,
) -> (u32, u32) {
    let vert_offset = vertices.len();

    let position = ui_frame.position;
    let size = ui_frame.size;

    vertices.push(Vertex2d {
        pos: ui_frame.position,
        uv: ui_frame.uvs[0],
        color: ui_frame.color,
    });

    vertices.push(Vertex2d {
        pos: vec2(position.x, position.y + size.y),
        uv: ui_frame.uvs[1],
        color: ui_frame.color,
    });

    vertices.push(Vertex2d {
        pos: vec2(position.x + size.x, position.y + size.y),
        uv: ui_frame.uvs[2],
        color: ui_frame.color,
    });

    vertices.push(Vertex2d {
        pos: vec2(position.x + size.x, position.y),
        uv: ui_frame.uvs[3],
        color: ui_frame.color,
    });

    indices.push(vert_offset as u16);
    indices.push(vert_offset as u16 + 1);
    indices.push(vert_offset as u16 + 2);

    indices.push(vert_offset as u16);
    indices.push(vert_offset as u16 + 2);
    indices.push(vert_offset as u16 + 3);

    (4, 6)
}

pub fn push_text_verts(
    window_extent: vk::Extent2D,
    ui_text: &UiText,
    glyph_atlas: &GlyphAtlas,
    vertices: &mut Vec<Vertex2d>,
    indices: &mut Vec<u16>,
) -> (u32, u32) {
    let glyphs = glyph_atlas.get_glyphs(&ui_text.text, ui_text.font_height);

    let mut vertex_count = 0;
    let mut index_count = 0;

    let mut pen_x = ui_text.position.x;
    let mut pen_y = ui_text.position.y;

    let mut pen_positions = Vec::with_capacity(glyphs.len());

    for glyph in glyphs.iter() {
        pen_positions.push((pen_x, pen_y));
        pen_x += glyph.advance.0;
        pen_y += glyph.advance.1;
    }

    for (i, glyph) in glyphs.iter().enumerate() {
        if glyph.is_empty {
            continue;
        }

        let glyph_position = pen_positions[i];
        let pos_x = glyph_position.0 as f32 + glyph.bitmap_left as f32;
        let pos_y = glyph_position.1 as f32 - glyph.bitmap_top as f32;

        let vert_offset = vertices.len();

        let glyph_x = glyph.rect.x as f32;
        let glyph_y = glyph.rect.y as f32;

        let glyph_width = glyph.rect.width as f32;
        let glyph_height = glyph.rect.height as f32;

        vertices.push(Vertex2d {
            pos: Vec2::new(pos_x, pos_y),
            uv: Vec2::new(glyph_x, glyph_y),
            color: Vec4::new(ui_text.color.x, ui_text.color.y, ui_text.color.z, 1.0),
        });

        vertices.push(Vertex2d {
            pos: Vec2::new(pos_x, pos_y + glyph_height),
            uv: Vec2::new(glyph_x, glyph_y + glyph_height),
            color: Vec4::new(ui_text.color.x, ui_text.color.y, ui_text.color.z, 1.0),
        });

        vertices.push(Vertex2d {
            pos: Vec2::new(pos_x + glyph_width, pos_y + glyph_height),
            uv: Vec2::new(glyph_x + glyph_width, glyph_y + glyph_height),
            color: Vec4::new(ui_text.color.x, ui_text.color.y, ui_text.color.z, 1.0),
        });

        vertices.push(Vertex2d {
            pos: Vec2::new(pos_x + glyph_width, pos_y),
            uv: Vec2::new(glyph_x + glyph_width, glyph_y),
            color: Vec4::new(ui_text.color.x, ui_text.color.y, ui_text.color.z, 1.0),
        });

        indices.push(vert_offset as u16);
        indices.push(vert_offset as u16 + 1);
        indices.push(vert_offset as u16 + 2);

        indices.push(vert_offset as u16);
        indices.push(vert_offset as u16 + 2);
        indices.push(vert_offset as u16 + 3);

        vertex_count += 4;
        index_count += 6;
    }

    (vertex_count, index_count)
}
