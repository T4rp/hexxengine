use ash::vk::{self, Rect2D};
use glam::{Vec2, Vec4, vec2};

use crate::{
    font_manager::{self, FontManager},
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
    font_manager: &FontManager,
    glyph_atlas: &GlyphAtlas,
    vertices: &mut Vec<Vertex2d>,
    indices: &mut Vec<u16>,
) -> (u32, u32) {
    let mut vertex_count = 0;
    let mut index_count = 0;

    let mut pen_x = ui_text.position.x as i32;
    let mut pen_y = ui_text.position.y as i32;

    for character in ui_text.text.chars() {
        let glyph_atlas_rect = glyph_atlas
            .get_glyph(ui_text.font, character as u64, ui_text.font_height)
            .unwrap();

        println!("{}", character);
        let glyph_data = font_manager
            .get_glyph(ui_text.font, character as u64, ui_text.font_height)
            .unwrap();

        pen_x += glyph_data.advance.0;
        pen_y -= glyph_data.advance.1;

        let pos_x = pen_x as f32 + glyph_data.bitmap_left as f32;
        let pos_y = pen_y as f32 - glyph_data.bitmap_top as f32;

        let vert_offset = vertices.len();

        let glyph_x = glyph_atlas_rect.rect.x as f32;
        let glyph_y = glyph_atlas_rect.rect.y as f32;

        let glyph_width = glyph_atlas_rect.rect.width as f32;
        let glyph_height = glyph_atlas_rect.rect.height as f32;

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
