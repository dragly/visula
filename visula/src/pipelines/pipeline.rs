use crate::SimulationRenderData;

pub trait Pipeline {
    fn render(&mut self, data: &mut SimulationRenderData);
}
