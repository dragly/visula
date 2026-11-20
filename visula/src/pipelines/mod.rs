pub mod arcs;
pub mod circles;
pub mod cylinders;
pub mod glyphs;
pub mod lines;
pub mod mesh;
pub mod polygons;
pub mod quad;
pub mod rects;
pub mod renderable;
pub mod spheres;
#[cfg(test)]
mod test_util;
pub mod torus;

pub use arcs::*;
pub use circles::*;
pub use cylinders::*;
pub use glyphs::*;
pub use lines::*;
pub use mesh::*;
pub use polygons::*;
pub use quad::*;
pub use rects::*;
pub use renderable::*;
pub use spheres::*;
pub use torus::*;

pub(crate) fn layouts_by_group<'a>(
    leading: &[&'a wgpu::BindGroupLayout],
    delegate: &'a [(u32, wgpu::BindGroupLayout)],
) -> Vec<Option<&'a wgpu::BindGroupLayout>> {
    let mut layouts: Vec<Option<&wgpu::BindGroupLayout>> =
        leading.iter().copied().map(Some).collect();
    for (group, layout) in delegate {
        let group = *group as usize;
        if layouts.len() <= group {
            layouts.resize(group + 1, None);
        }
        layouts[group] = Some(layout);
    }
    layouts
}
