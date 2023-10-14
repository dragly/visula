use crate::RenderData;

pub trait Renderable {
    fn render(&self, render_data: &mut RenderData) -> ();
}
