use glam::Vec3;

pub fn hsv_to_rgb(hue: f32, sat: f32, value: f32) -> Vec3 {
    let c = value * sat;
    let hp = hue / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());

    let (r, g, b) = if (0.0..1.0).contains(&hp) {
        (c, x, 0.0)
    } else if (1.0..2.0).contains(&hp) {
        (x, c, 0.0)
    } else if (2.0..3.0).contains(&hp) {
        (0.0, c, x)
    } else if (3.0..4.0).contains(&hp) {
        (0.0, x, c)
    } else if (4.0..5.0).contains(&hp) {
        (x, 0.0, c)
    } else if (5.0..6.0).contains(&hp) {
        (c, 0.0, x)
    } else {
        (0.0, 0.0, 0.0)
    };

    let m = value - c;

    Vec3::new(r + m, g + m, b + m)
}
