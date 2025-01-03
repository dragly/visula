use naga::back::wgsl::WriterFlags;
use naga::valid::ValidationFlags;
use naga::{Module, ShaderStage};

use crate::{BindingBuilder, Expression};

macro_rules! entry_point {
    ($module: ident, $shader_stage: expr) => {
        $module
            .entry_points
            .iter_mut()
            .find(|e| e.stage == $shader_stage)
            .expect(&format!("Could not find entry point in shader"))
    };
}

pub fn inject(
    module: &mut Module,
    binding_builder: &mut BindingBuilder,
    shader_stage: ShaderStage,
    variable_name: &str,
    fields: &[Expression],
) {
    let variable = entry_point!(module, shader_stage)
        .function
        .local_variables
        .fetch_if(|variable| variable.name == Some(variable_name.into()))
        .unwrap_or_else(|| panic!("Could not find variable with name '{variable_name}' in shader"));
    let variable_expression = entry_point!(module, shader_stage)
        .function
        .expressions
        .fetch_if(|expression| match expression {
            naga::Expression::LocalVariable(v) => v == &variable,
            _ => false,
        })
        .unwrap();

    let fields_setup = fields
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let expression = value.setup(module, binding_builder);
            let access_index = entry_point!(module, shader_stage)
                .function
                .expressions
                .append(
                    naga::Expression::AccessIndex {
                        index: index as u32,
                        base: variable_expression,
                    },
                    naga::Span::default(),
                );
            ::naga::Statement::Store {
                pointer: access_index,
                value: expression,
            }
        })
        .collect();
    let mut new_body = ::naga::Block::from_vec(fields_setup);

    for (statement, span) in entry_point!(module, shader_stage)
        .function
        .body
        .span_iter_mut()
    {
        new_body.push(
            statement.clone(),
            match span {
                Some(s) => *s,
                None => naga::Span::default(),
            },
        );
    }
    entry_point!(module, shader_stage).function.body = new_body;

    let info =
        naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
            .validate(module)
            .unwrap();
    let output_str = naga::back::wgsl::write_string(module, &info, WriterFlags::all()).unwrap();
    log::debug!("Resulting lines shader code:\n{}", output_str);
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    #[test]
    fn test_inject() {
        env_logger::try_init().unwrap();
        let mut module =
            naga::front::wgsl::parse_str(include_str!("./shaders/basic.wgsl")).unwrap();
        let vertex_fields: Vec<Expression> = vec![
            Vec3::new(0.0, 0.0, 0.0).into(),
            Vec3::new(1.0, 0.0, 0.0).into(),
            1.0.into(),
        ];
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 2);
        inject(
            &mut module,
            &mut binding_builder,
            ShaderStage::Vertex,
            "line_vertex",
            &vertex_fields,
        );

        let mut binding_builder = BindingBuilder::new(&module, "fs_main", 2);
        let fragment_fields: Vec<Expression> = vec![Vec3::new(1.0, 1.0, 0.0).into()];
        inject(
            &mut module,
            &mut binding_builder,
            ShaderStage::Fragment,
            "line_fragment",
            &fragment_fields,
        );
    }
}
