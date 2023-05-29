use crate::RenderData;

pub trait Pipeline {
    fn render(&mut self, data: &mut RenderData);
}
