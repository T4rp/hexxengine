use glam::Vec3;

pub fn hsv_to_rgb(hue: f32, sat: f32, value: f32) -> Vec3 {
    let c = value * sat;
    let hp = hue / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());

    let (r, g, b) = if hp >= 0.0 && hp < 1.0 {
        (c, x, 0.0)
    } else if hp >= 1.0 && hp < 2.0 {
        (x, c, 0.0)
    } else if hp >= 2.0 && hp < 3.0 {
        (0.0, c, x)
    } else if hp >= 3.0 && hp < 4.0 {
        (0.0, x, c)
    } else if hp >= 4.0 && hp < 5.0 {
        (x, 0.0, c)
    } else if hp >= 5.0 && hp < 6.0 {
        (c, 0.0, x)
    } else {
        (0.0, 0.0, 0.0)
    };

    let m = value - c;

    Vec3::new(r + m, g + m, b + m)
}
