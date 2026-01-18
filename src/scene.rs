use glam::{Mat4, Quat, Vec3, vec4};

pub struct Camera {
    pub position: Vec3,
    pub orientation: Quat,
    pub fov: f32,
}

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
}

pub struct Lighting {
    pub sun_direction: Vec3,
    pub sun_color: Vec3,
    pub sun_power: f32,
    pub ambient_color: Vec3,
}

pub struct MeshNode {
    pub position: Vec3,
    pub orientation: Quat,
    pub color: Vec3,
    pub mesh_id: u32,
    pub material_id: u32,
}

pub struct RenderScene {
    pub camera: Camera,
    pub meshes: Vec<MeshNode>,
    pub lighting: Lighting,
    pub are_meshes_dirty: bool,
}
