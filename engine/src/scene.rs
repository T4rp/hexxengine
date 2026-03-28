use std::{
    any::{self, Any, TypeId},
    borrow::Cow,
    collections::HashMap,
    rc::Rc,
};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles, vec2, vec4};
use thunderdome::{Arena, Index};

use crate::{
    renderer::{
        buffer_objects::{scene2d::Vertex2d, scene3d::MeshVertex},
        renderer::MeshHandle,
    },
    text::GlyphAtlas,
};

pub struct Camera {
    pub position: Vec3,
    pub orientation: Quat,
    pub fov: f32,
}

const NDC_CORNERS: &[Vec4] = &[
    vec4(-1.0, -1.0, 1.0, 1.0),
    vec4(1.0, -1.0, 1.0, 1.0),
    vec4(-1.0, 1.0, 1.0, 1.0),
    vec4(1.0, 1.0, 1.0, 1.0),
    vec4(-1.0, -1.0, 0.0, 1.0),
    vec4(1.0, -1.0, 0.0, 1.0),
    vec4(-1.0, 1.0, 0.0, 1.0),
    vec4(1.0, 1.0, 0.0, 1.0),
];

impl Camera {
    pub fn new(position: Vec3, orientation: Quat, fov: f32) -> Self {
        Self {
            position,
            orientation,
            fov,
        }
    }

    pub fn calc_perspective_matrices(&self, aspect_ratio: f32) -> (Mat4, Mat4) {
        let vertical_fov = 2.0 * (self.fov.to_radians() * 0.5).tan().atan2(aspect_ratio);

        let mut proj = Mat4::perspective_infinite_reverse_rh(vertical_fov, aspect_ratio, 1.0);
        proj.y_axis *= vec4(1.0, -1.0, 1.0, 1.0);

        let view = Mat4::from_rotation_translation(self.orientation, self.position).inverse();

        (proj, view)
    }

    pub fn calc_frustrum_corners(&self, aspect_ratio: f32, near: f32, far: f32) -> [Vec3; 8] {
        let vertical_fov = 2.0 * (self.fov.to_radians() * 0.5).tan().atan2(aspect_ratio);

        let mut proj = Mat4::perspective_rh(vertical_fov, aspect_ratio, near, far);
        proj.y_axis *= vec4(1.0, -1.0, 1.0, 1.0);

        let view = Mat4::from_rotation_translation(self.orientation, self.position).inverse();

        let mut corners = [Vec3::ZERO; 8];

        let inv_pv = (proj * view).inverse();
        for i in 0..8 {
            let corner4 = inv_pv * NDC_CORNERS[i];
            corners[i] = corner4.xyz() / corner4.w
        }

        corners
    }
}

pub struct Lighting {
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub sun_power: f32,
    pub ambient_color: Vec3,
    pub skybox_id: u32,
}

#[derive(Clone)]
pub struct MeshNode {
    pub position: Vec3,
    pub orientation: Quat,
    pub size: Vec3,
    pub color: Vec3,
    pub opacity: f32,
    pub mesh_id: MeshHandle,
    pub material_id: u32,
}

pub struct UiFrame {
    pub position: Vec2,
    pub size: Vec2,
    pub color: Vec4,
    pub texture_id: u32,
    pub uvs: [Vec2; 4],
}

impl UiFrame {
    pub fn new(position: Vec2, size: Vec2, texture_id: u32) -> Self {
        Self {
            position,
            size,
            color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            texture_id,
            uvs: [
                vec2(0.0, 0.0),
                vec2(0.0, 1.0),
                vec2(1.0, 1.0),
                vec2(1.0, 0.0),
            ],
        }
    }

    pub fn push_verts(&self, vertices: &mut Vec<Vertex2d>, indices: &mut Vec<u16>) -> (u32, u32) {
        let vert_offset = vertices.len();

        vertices.push(Vertex2d {
            pos: self.position,
            uv: self.uvs[0],
            color: self.color,
        });

        vertices.push(Vertex2d {
            pos: vec2(self.position.x, self.position.y + self.size.y),
            uv: self.uvs[1],
            color: self.color,
        });

        vertices.push(Vertex2d {
            pos: vec2(self.position.x + self.size.x, self.position.y + self.size.y),
            uv: self.uvs[2],
            color: self.color,
        });

        vertices.push(Vertex2d {
            pos: vec2(self.position.x + self.size.x, self.position.y),
            uv: self.uvs[3],
            color: self.color,
        });

        indices.push(vert_offset as u16);
        indices.push(vert_offset as u16 + 1);
        indices.push(vert_offset as u16 + 2);

        indices.push(vert_offset as u16);
        indices.push(vert_offset as u16 + 2);
        indices.push(vert_offset as u16 + 3);

        (4, 6)
    }
}

pub struct UiText {
    pub position: Vec2,
    pub font_height: u32,
    pub text: Cow<'static, str>,
    pub color: Vec3,
}

impl UiText {
    pub fn new(position: Vec2, height: u32, text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            position,
            font_height: height,
            text: text.into(),
            color: Vec3::ZERO,
        }
    }

    pub fn push_verts(
        &self,
        glyph_atlas: &GlyphAtlas,
        vertices: &mut Vec<Vertex2d>,
        indices: &mut Vec<u16>,
    ) -> (u32, u32) {
        let glyphs = glyph_atlas.get_glyphs(&self.text, self.font_height);

        let mut vertex_count = 0;
        let mut index_count = 0;

        let mut pen_x = self.position.x;
        let mut pen_y = self.position.y;

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
                color: Vec4::new(self.color.x, self.color.y, self.color.z, 1.0),
            });

            vertices.push(Vertex2d {
                pos: Vec2::new(pos_x, pos_y + glyph_height),
                uv: Vec2::new(glyph_x, glyph_y + glyph_height),
                color: Vec4::new(self.color.x, self.color.y, self.color.z, 1.0),
            });

            vertices.push(Vertex2d {
                pos: Vec2::new(pos_x + glyph_width, pos_y + glyph_height),
                uv: Vec2::new(glyph_x + glyph_width, glyph_y + glyph_height),
                color: Vec4::new(self.color.x, self.color.y, self.color.z, 1.0),
            });

            vertices.push(Vertex2d {
                pos: Vec2::new(pos_x + glyph_width, pos_y),
                uv: Vec2::new(glyph_x + glyph_width, glyph_y),
                color: Vec4::new(self.color.x, self.color.y, self.color.z, 1.0),
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
}

pub enum UiDraw {
    Frame(UiFrame),
    Text(UiText),
}

pub struct RenderScene {
    pub camera: Camera,
    pub meshes: Vec<MeshNode>,
    pub ui: Vec<UiDraw>,
    pub lighting: Lighting,
}

impl RenderScene {
    pub fn new(camera: Camera, lighting: Lighting) -> Self {
        Self {
            camera,
            meshes: Vec::new(),
            ui: Vec::new(),
            lighting,
        }
    }

    pub fn push_ui_frame(&mut self, frame: UiFrame) {
        self.ui.push(UiDraw::Frame(frame));
    }

    pub fn push_ui_text(&mut self, ui_text: UiText) {
        self.ui.push(UiDraw::Text(ui_text));
    }
}

pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u16>,
}

struct World {
    pub storages: HashMap<TypeId, Box<dyn Any>>,
    pub entities: Arena<Entity>,
}

impl World {
    pub fn new() -> Self {
        let storages = HashMap::new();
        let entities = Arena::new();

        Self { storages, entities }
    }

    pub fn add_entity(&mut self) -> Index {
        self.entities.insert(Entity::new())
    }

    pub fn add_component<T: EntityComponent + 'static>(
        &mut self,
        entity_index: Index,
        component: T,
    ) -> Option<thunderdome::Index> {
        let Some(entity) = self.entities.get(entity_index) else {
            return None;
        };

        let type_id = TypeId::of::<T>();
        if entity
            .components
            .iter()
            .find(|comp| comp.0 == type_id)
            .is_some()
        {
            return None;
        };

        let storage = self.get_storage_mut::<T>().unwrap();
        let component_index = storage.insert(component);

        self.entities
            .get_mut(entity_index)
            .unwrap()
            .components
            .push((type_id, component_index));

        Some(component_index)
    }

    pub fn get_component<T: EntityComponent + 'static>(&self, entity_index: Index) -> Option<&T> {
        let Some(entity) = self.entities.get(entity_index) else {
            return None;
        };

        let type_id = TypeId::of::<T>();

        let component_index = entity
            .components
            .iter()
            .find_map(|comp| if comp.0 == type_id { Some(comp) } else { None });

        let component_index = match component_index {
            Some(ci) => ci,
            None => return None,
        };

        if entity
            .components
            .iter()
            .find(|comp| comp.0 == type_id)
            .is_none()
        {
            return None;
        };

        let Some(storage) = self.get_storage_ref::<T>() else {
            return None;
        };

        storage.get(component_index.1)
    }

    pub fn get_storage_mut<T: EntityComponent + 'static>(&mut self) -> Option<&mut Arena<T>> {
        let type_id = TypeId::of::<T>();
        let storage = self
            .storages
            .entry(type_id)
            .or_insert_with(|| Box::new(Arena::<T>::new()));

        storage.downcast_mut::<thunderdome::Arena<T>>()
    }

    pub fn get_storage_ref<T: EntityComponent + 'static>(&self) -> Option<&thunderdome::Arena<T>> {
        let type_id = TypeId::of::<T>();
        let Some(storage) = self.storages.get(&type_id) else {
            return None;
        };

        storage.downcast_ref::<thunderdome::Arena<T>>()
    }
}

trait EntityComponent {}

pub struct Entity {
    pub parent: thunderdome::Index,
    pub children: Vec<thunderdome::Index>,
    pub components: Vec<(TypeId, thunderdome::Index)>,
}

impl Entity {
    fn new() -> Self {
        let parent = thunderdome::Index::DANGLING;
        let children = Vec::new();
        let components = Vec::new();

        Self {
            parent,
            children,
            components,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::scene::{Entity, EntityComponent, World};

    #[derive(Debug)]
    struct TestComponent {
        foo: u32,
    }
    impl EntityComponent for TestComponent {}

    #[test]
    fn add_component() {
        let mut world = World::new();
        let entity = world.add_entity();
        world
            .add_component(entity, TestComponent { foo: 100 })
            .unwrap();
    }

    #[test]
    fn get_component() {
        let mut world = World::new();
        let entity = world.add_entity();

        let comp_index = world
            .add_component(entity, TestComponent { foo: 100 })
            .unwrap();

        let component = world.get_component::<TestComponent>(comp_index).unwrap();
        assert_eq!(component.foo, 100)
    }
}
