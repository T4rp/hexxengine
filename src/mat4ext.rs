use glam::{Mat4, Vec4};

/// Creates a right-handed orthographic projection matrix with `[0,1]` depth range.
///
/// Useful to map a right-handed coordinate system to the normalized device coordinates that WebGPU/Direct3D/Metal expect.
#[inline]
#[must_use]
pub fn orthographic_rh(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    let rcp_width = 1.0 / (right - left);
    let rcp_height = 1.0 / (top - bottom);
    let r = 1.0 / (near - far);
    Mat4::from_cols(
        Vec4::new(rcp_width + rcp_width, 0.0, 0.0, 0.0),
        Vec4::new(0.0, rcp_height + rcp_height, 0.0, 0.0),
        Vec4::new(0.0, 0.0, r, 0.0),
        Vec4::new(
            -(left + right) * rcp_width,
            -(top + bottom) * rcp_height,
            r * near,
            1.0,
        ),
    )
}
