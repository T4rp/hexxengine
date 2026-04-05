use glam::{Vec2, Vec4, vec2};

use crate::{
    renderer::buffer_objects::scene2d::Vertex2d,
    scene::{UiFrame, UiText},
    text::GlyphAtlas,
};

pub fn push_frame_verts(
    ui_frame: &UiFrame,
    vertices: &mut Vec<Vertex2d>,
    indices: &mut Vec<u16>,
) -> (u32, u32) {
    let vert_offset = vertices.len();

    vertices.push(Vertex2d {
        pos: ui_frame.position,
        uv: ui_frame.uvs[0],
        color: ui_frame.color,
    });

    vertices.push(Vertex2d {
        pos: vec2(ui_frame.position.x, ui_frame.position.y + ui_frame.size.y),
        uv: ui_frame.uvs[1],
        color: ui_frame.color,
    });

    vertices.push(Vertex2d {
        pos: vec2(
            ui_frame.position.x + ui_frame.size.x,
            ui_frame.position.y + ui_frame.size.y,
        ),
        uv: ui_frame.uvs[2],
        color: ui_frame.color,
    });

    vertices.push(Vertex2d {
        pos: vec2(ui_frame.position.x + ui_frame.size.x, ui_frame.position.y),
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

    for glyph in glyphs {
        if glyph.is_empty {
            pen_x += glyph.advance.0 as f32;
            pen_y += glyph.advance.1 as f32;
            continue;
        }

        let vert_offset = vertices.len();

        let pos_x = pen_x + glyph.bitmap_left as f32;
        let pos_y = pen_y - glyph.bitmap_top as f32;

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

        pen_x += glyph.advance.0 as f32;
        pen_y += glyph.advance.1 as f32;
    }

    (vertex_count, index_count)
}
