use crate::{RenderData, ShadowRenderData};

pub trait Renderable {
    fn render(&self, render_data: &mut RenderData);
    fn render_shadow(&self, _shadow_data: &mut ShadowRenderData) {}
}
