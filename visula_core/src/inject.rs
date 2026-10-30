use naga::back::wgsl::WriterFlags;
use naga::valid::ValidationFlags;
use naga::{Arena, Block, Handle, Module, Statement};

use crate::error::ShaderError;
use crate::value::{resolve_type, type_name};
use crate::{BindingBuilder, Expression};

fn check_field_type(
    module: &Module,
    binding_builder: &mut BindingBuilder,
    variable: naga::Handle<naga::LocalVariable>,
    variable_name: &str,
    index: usize,
    expression: naga::Handle<naga::Expression>,
) -> Result<(), ShaderError> {
    let function = &module.entry_points[binding_builder.entry_point_index].function;
    let naga::TypeInner::Struct { members, .. } =
        &module.types[function.local_variables[variable].ty].inner
    else {
        return Ok(());
    };
    let Some(member) = members.get(index) else {
        return Ok(());
    };
    let expected = &module.types[member.ty].inner;
    let found = resolve_type(expression, module, binding_builder)?;
    if found == *expected {
        return Ok(());
    }
    Err(ShaderError::FieldTypeMismatch {
        field: format!(
            "{variable_name}.{}",
            member.name.as_deref().unwrap_or_default()
        ),
        expected: type_name(expected, module),
        found: type_name(&found, module),
    })
}

fn field_stores(
    module: &mut Module,
    binding_builder: &mut BindingBuilder,
    variable_name: &str,
    fields: &[Expression],
) -> Result<(Handle<naga::Expression>, Vec<Statement>), ShaderError> {
    binding_builder.shared.clear();
    let function = &module.entry_points[binding_builder.entry_point_index].function;
    let variable = function
        .local_variables
        .fetch_if(|variable| variable.name == Some(variable_name.into()))
        .ok_or_else(|| ShaderError::VariableNotFound(variable_name.to_string()))?;
    let variable_expression = function
        .expressions
        .fetch_if(|expression| match expression {
            naga::Expression::LocalVariable(v) => v == &variable,
            _ => false,
        })
        .ok_or_else(|| ShaderError::VariableNotFound(variable_name.to_string()))?;

    let stores = fields
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let expression = value.setup(module, binding_builder)?;
            check_field_type(
                module,
                binding_builder,
                variable,
                variable_name,
                index,
                expression,
            )?;
            let access_index = module.entry_points[binding_builder.entry_point_index]
                .function
                .expressions
                .append(
                    naga::Expression::AccessIndex {
                        index: index as u32,
                        base: variable_expression,
                    },
                    naga::Span::default(),
                );
            Ok(Statement::Store {
                pointer: access_index,
                value: expression,
            })
        })
        .collect::<Result<Vec<_>, ShaderError>>()?;
    let mut statements = std::mem::take(&mut binding_builder.pending_statements);
    statements.extend(stores);
    Ok((variable_expression, statements))
}

fn validate(module: &Module) -> Result<(), ShaderError> {
    let info = naga::valid::Validator::new(
        ValidationFlags::EXPRESSIONS,
        naga::valid::Capabilities::all(),
    )
    .validate(module)
    .map_err(Box::new)?;
    let output_str = naga::back::wgsl::write_string(module, &info, WriterFlags::all())?;
    log::debug!("Resulting shader code:\n{output_str}");
    Ok(())
}

pub fn inject(
    module: &mut Module,
    binding_builder: &mut BindingBuilder,
    variable_name: &str,
    fields: &[Expression],
) -> Result<(), ShaderError> {
    let (_, statements) = field_stores(module, binding_builder, variable_name, fields)?;
    let function = &mut module.entry_points[binding_builder.entry_point_index].function;
    let mut body = Block::from_vec(statements);
    for (statement, span) in function.body.span_iter() {
        body.push(statement.clone(), *span);
    }
    function.body = body;
    validate(module)
}

pub fn inject_before_use(
    module: &mut Module,
    binding_builder: &mut BindingBuilder,
    variable_name: &str,
    fields: &[Expression],
) -> Result<(), ShaderError> {
    let (variable, statements) = field_stores(module, binding_builder, variable_name, fields)?;
    let function = &mut module.entry_points[binding_builder.entry_point_index].function;
    let first_use = function
        .body
        .iter()
        .position(|statement| {
            uses(statement, variable, &function.expressions)
                || matches!(statement, Statement::Return { .. })
        })
        .unwrap_or(function.body.len());
    let mut body = Block::new();
    for (index, (statement, span)) in function.body.span_iter().enumerate() {
        if index == first_use {
            for statement in &statements {
                body.push(statement.clone(), naga::Span::default());
            }
        }
        body.push(statement.clone(), *span);
    }
    if first_use == function.body.len() {
        for statement in statements {
            body.push(statement, naga::Span::default());
        }
    }
    function.body = body;
    validate(module)
}

fn uses(
    statement: &Statement,
    variable: Handle<naga::Expression>,
    expressions: &Arena<naga::Expression>,
) -> bool {
    let block_uses = |block: &Block| {
        block
            .iter()
            .any(|statement| uses(statement, variable, expressions))
    };
    match statement {
        Statement::Emit(range) => range.clone().any(|handle| match expressions[handle] {
            naga::Expression::Load { pointer } => pointer == variable,
            naga::Expression::Access { base, .. } | naga::Expression::AccessIndex { base, .. } => {
                base == variable
            }
            _ => false,
        }),
        Statement::Store { pointer, value } => *pointer == variable || *value == variable,
        Statement::Call { arguments, .. } => arguments.contains(&variable),
        Statement::Block(block) => block_uses(block),
        Statement::If { accept, reject, .. } => block_uses(accept) || block_uses(reject),
        Statement::Loop {
            body, continuing, ..
        } => block_uses(body) || block_uses(continuing),
        Statement::Switch { cases, .. } => cases.iter().any(|case| block_uses(&case.body)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;

    #[test]
    fn test_inject() {
        let _ = env_logger::try_init();
        let mut module =
            naga::front::wgsl::parse_str(include_str!("./shaders/basic.wgsl")).unwrap();
        let vertex_fields: Vec<Expression> = vec![
            Vec3::new(0.0, 0.0, 0.0).into(),
            Vec3::new(1.0, 0.0, 0.0).into(),
            1.0.into(),
        ];
        let mut binding_builder = BindingBuilder::new(&module, "vs_main", 2).unwrap();
        inject(
            &mut module,
            &mut binding_builder,
            "line_vertex",
            &vertex_fields,
        )
        .unwrap();

        let mut binding_builder = BindingBuilder::new(&module, "fs_main", 2).unwrap();
        let fragment_fields: Vec<Expression> = vec![Vec3::new(1.0, 1.0, 0.0).into()];
        inject(
            &mut module,
            &mut binding_builder,
            "line_fragment",
            &fragment_fields,
        )
        .unwrap();
    }
}
