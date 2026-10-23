use glam::Vec3;

use crate::{mix, Expression};

/// Converts an sRGB hex color, like `0x6E8FC9` from CSS or a design tool,
/// to the linear values that Visula expects.
pub fn srgb(hex: u32) -> Vec3 {
    srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Converts 8-bit sRGB channels to the linear values that Visula expects.
pub fn srgb_u8(r: u8, g: u8, b: u8) -> Vec3 {
    Vec3::new(r as f32, g as f32, b as f32).map(|channel| srgb_channel_to_linear(channel / 255.0))
}

fn srgb_channel_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts an sRGB-encoded `vec3` expression to linear values on the GPU,
/// for materials that deliberately compute their colors in sRGB space.
pub fn srgb_to_linear(color: impl Into<Expression>) -> Expression {
    let color = color.into().shared_with_name("srgb");
    let low = &color / 12.92;
    let high = ((&color + 0.055) / 1.055).pow(Vec3::splat(2.4));
    mix(low, high, color.step(Vec3::splat(0.04045)))
}

#[cfg(test)]
mod tests {
    use naga::ShaderStage;

    use super::*;
    use crate::test_util::{device, lower_to_wgsl};
    use crate::InstanceBuffer;

    #[test]
    fn hex_and_u8_agree() {
        assert_eq!(srgb(0x6E8FC9), srgb_u8(110, 143, 201));
    }

    #[test]
    fn known_colors() {
        for (hex, linear) in [
            (0x6E8FC9, Vec3::new(0.15593, 0.27468, 0.58408)),
            (0xD8865A, Vec3::new(0.68669, 0.23840, 0.10224)),
            (0xF6F5F0, Vec3::new(0.92158, 0.91310, 0.87137)),
            (0x000000, Vec3::ZERO),
            (0xFFFFFF, Vec3::ONE),
        ] {
            assert!(srgb(hex).abs_diff_eq(linear, 1e-5), "{hex:06X}");
        }
    }

    #[test]
    fn expression() {
        let device = device();
        let t = InstanceBuffer::<f32>::new(&device).instance();
        let wgsl = lower_to_wgsl(
            ShaderStage::Fragment,
            &[("vec3<f32>", srgb_to_linear(crate::vec3(&t, 0.5, 1.0)))],
        );
        insta::assert_snapshot!(wgsl);
    }
}
