use ash::vk::{self};
use glam::{Vec2, Vec4, vec2};

use crate::{
    renderer::scene2d::Vertex2d,
    scene::{UiFrame, UiText},
    text::{FontManager, GlyphAtlas},
};

pub fn push_frame_verts(
    _window_extent: vk::Extent2D,
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
    _window_extent: vk::Extent2D,
    ui_text: &UiText,
    font_manager: &FontManager,
    glyph_atlas: &GlyphAtlas,
    vertices: &mut Vec<Vertex2d>,
    indices: &mut Vec<u16>,
) -> (u32, u32) {
    let mut vertex_count = 0;
    let mut index_count = 0;

    let font = font_manager.get_font_ref(ui_text.font);

    for character in ui_text.glyph_positions.iter() {
        let glyph_atlas_rect = glyph_atlas
            .get_glyph(ui_text.font, character.glyph, ui_text.font_height)
            .unwrap();

        let glyph_data = font
            .get_glyph(character.glyph, ui_text.font_height)
            .unwrap();

        let pos_x = ui_text.position.x + character.offset.x + glyph_data.bitmap_left as f32;
        let pos_y = ui_text.position.y + character.offset.y - glyph_data.bitmap_top as f32;

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
