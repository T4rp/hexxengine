use glam::{
    Mat4, Quat, Vec3, Vec4, Vec4Swizzles, camera::rh::proj::vulkan as vk_camera_proj, vec4,
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

    /// return projection and view matrix
    pub fn calc_perspective_matrices(&self, aspect_ratio: f32) -> (Mat4, Mat4) {
        let vertical_fov = 2.0 * (self.fov.to_radians() * 0.5).tan().atan2(aspect_ratio);

        let proj = vk_camera_proj::perspective_infinite_reverse(vertical_fov, aspect_ratio, 1.0);
        let view = Mat4::from_rotation_translation(self.orientation, self.position).inverse();

        (proj, view)
    }

    pub fn screen_to_world_ray(
        &self,
        x_pos: f32,
        y_pos: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> (Vec3, Vec3) {
        let x = (2.0 * x_pos) / screen_width - 1.0;
        let y = (2.0 * y_pos) / screen_height - 1.0;

        let (proj, view) = self.calc_perspective_matrices(screen_width / screen_height);
        let inv_proj_view = (proj * view).inverse();

        let world_near = inv_proj_view * Vec4::new(x, y, 1.0, 1.0);
        let world_near = world_near.xyz() / world_near.w;

        let direction = (world_near - self.position).normalize();

        return (self.position, direction);
    }

    pub fn world_to_screen_space(
        &self,
        position: Vec3,
        screen_width: f32,
        screen_height: f32,
    ) -> Vec3 {
        let (proj, view) = self.calc_perspective_matrices(screen_width / screen_height);

        let world_space_ps = proj * view * position.extend(1.0);
        world_space_ps.xyz() / world_space_ps.w
    }

    pub fn calc_frustrum_corners(&self, aspect_ratio: f32, near: f32, far: f32) -> [Vec3; 8] {
        let vertical_fov = 2.0 * (self.fov.to_radians() * 0.5).tan().atan2(aspect_ratio);

        let proj = vk_camera_proj::perspective(vertical_fov, aspect_ratio, near, far);
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
