use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles, vec4};

use crate::{mesh::MeshVertex, vulkan::MeshHandle};

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

pub struct RenderScene {
    pub camera: Camera,
    pub meshes: Vec<MeshNode>,
    pub lighting: Lighting,
}

pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u16>,
}
