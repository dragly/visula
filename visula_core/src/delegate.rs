use crate::error::ShaderError;
use crate::BindingBuilder;

pub trait Delegate {
    fn inject(
        &self,
        shader_variable_name: &str,
        module: &mut naga::Module,
        binding_builder: &mut BindingBuilder,
    ) -> Result<(), ShaderError>;
}
