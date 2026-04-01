use naga::back::wgsl::WriterFlags;
use naga::valid::ValidationFlags;
use naga::Module;

use crate::error::ShaderError;
use crate::{BindingBuilder, Expression};

macro_rules! entry_point {
    ($module: ident, $shader_stage: expr) => {
        $module
            .entry_points
            .iter_mut()
            .find(|e| e.stage == $shader_stage)
            .ok_or_else(|| ShaderError::EntryPointNotFound(format!("{:?}", $shader_stage)))?
    };
}

pub fn inject(
    module: &mut Module,
    binding_builder: &mut BindingBuilder,
    variable_name: &str,
    fields: &[Expression],
) -> Result<(), ShaderError> {
    let variable = entry_point!(module, binding_builder.shader_stage)
        .function
        .local_variables
        .fetch_if(|variable| variable.name == Some(variable_name.into()))
        .ok_or_else(|| ShaderError::VariableNotFound(variable_name.to_string()))?;
    let variable_expression = entry_point!(module, binding_builder.shader_stage)
        .function
        .expressions
        .fetch_if(|expression| match expression {
            naga::Expression::LocalVariable(v) => v == &variable,
            _ => false,
        })
        .ok_or_else(|| ShaderError::VariableNotFound(variable_name.to_string()))?;

    let fields_setup = fields
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let expression = value.setup(module, binding_builder);
            let access_index = entry_point!(module, binding_builder.shader_stage)
                .function
                .expressions
                .append(
                    naga::Expression::AccessIndex {
                        index: index as u32,
                        base: variable_expression,
                    },
                    naga::Span::default(),
                );
            Ok(::naga::Statement::Store {
                pointer: access_index,
                value: expression,
            })
        })
        .collect::<Result<Vec<_>, ShaderError>>()?;
    let mut pending = Vec::new();
    pending.append(&mut binding_builder.pending_statements);
    let mut new_body = ::naga::Block::from_vec(pending);
    for store in fields_setup {
        new_body.push(store, naga::Span::default());
    }

    for (statement, span) in entry_point!(module, binding_builder.shader_stage)
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
    entry_point!(module, binding_builder.shader_stage)
        .function
        .body = new_body;

    let info =
        naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
            .validate(module)
            .map_err(Box::new)?;
    let output_str = naga::back::wgsl::write_string(module, &info, WriterFlags::all())?;
    log::debug!("Resulting shader code:\n{output_str}");
    Ok(())
}

pub fn inject_before_return(
    module: &mut Module,
    binding_builder: &mut BindingBuilder,
    variable_name: &str,
    fields: &[Expression],
) -> Result<(), ShaderError> {
    let variable = entry_point!(module, binding_builder.shader_stage)
        .function
        .local_variables
        .fetch_if(|variable| variable.name == Some(variable_name.into()))
        .ok_or_else(|| ShaderError::VariableNotFound(variable_name.to_string()))?;
    let variable_expression = entry_point!(module, binding_builder.shader_stage)
        .function
        .expressions
        .fetch_if(|expression| match expression {
            naga::Expression::LocalVariable(v) => v == &variable,
            _ => false,
        })
        .ok_or_else(|| ShaderError::VariableNotFound(variable_name.to_string()))?;

    let fields_setup = fields
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let expression = value.setup(module, binding_builder);
            let access_index = entry_point!(module, binding_builder.shader_stage)
                .function
                .expressions
                .append(
                    naga::Expression::AccessIndex {
                        index: index as u32,
                        base: variable_expression,
                    },
                    naga::Span::default(),
                );
            Ok(::naga::Statement::Store {
                pointer: access_index,
                value: expression,
            })
        })
        .collect::<Result<Vec<_>, ShaderError>>()?;

    let original_body = &entry_point!(module, binding_builder.shader_stage)
        .function
        .body;

    // Collect all non-return statements from the original body
    let mut new_body = naga::Block::new();
    for (statement, span) in original_body.span_iter() {
        if !matches!(statement, naga::Statement::Return { .. }) {
            new_body.push(statement.clone(), *span);
        }
    }

    // Insert pending statements (e.g. function calls) then field stores
    for stmt in binding_builder.pending_statements.drain(..) {
        new_body.push(stmt, naga::Span::default());
    }
    for store in fields_setup {
        new_body.push(store, naga::Span::default());
    }

    // Build a fresh return that re-loads from the variable AFTER the stores.
    // The original Return's expressions were pre-computed in an earlier Emit,
    // so they read zero. We need new Load expressions that execute after our stores.
    {
        let ep = &mut module.entry_points[binding_builder.entry_point_index];
        let var_ty = ep.function.local_variables[variable].ty;
        let struct_members = match &module.types[var_ty].inner {
            naga::TypeInner::Struct { members, .. } => members.clone(),
            _ => panic!("inject_before_return target must be a struct"),
        };

        // Re-load the first field (color) from the struct
        let new_access = ep.function.expressions.append(
            naga::Expression::AccessIndex {
                base: variable_expression,
                index: 0,
            },
            naga::Span::default(),
        );
        let new_load = ep.function.expressions.append(
            naga::Expression::Load {
                pointer: new_access,
            },
            naga::Span::default(),
        );

        // Emit the new load
        new_body.push(
            naga::Statement::Emit(naga::Range::new_from_bounds(new_access, new_load)),
            naga::Span::default(),
        );

        // Determine the return type. Fragment shaders return vec4<f32>.
        let field_ty = &module.types[struct_members[0].ty].inner;
        let return_value = match field_ty {
            naga::TypeInner::Vector {
                size: naga::VectorSize::Tri,
                ..
            } => {
                // vec3 field → compose vec4(field, 1.0)
                let one = ep.function.expressions.append(
                    naga::Expression::Literal(naga::Literal::F32(1.0)),
                    naga::Span::default(),
                );
                let vec4_type = module.types.insert(
                    naga::Type {
                        name: None,
                        inner: naga::TypeInner::Vector {
                            scalar: naga::Scalar {
                                kind: naga::ScalarKind::Float,
                                width: 4,
                            },
                            size: naga::VectorSize::Quad,
                        },
                    },
                    naga::Span::default(),
                );
                let compose = ep.function.expressions.append(
                    naga::Expression::Compose {
                        ty: vec4_type,
                        components: vec![new_load, one],
                    },
                    naga::Span::default(),
                );
                new_body.push(
                    naga::Statement::Emit(naga::Range::new_from_bounds(one, compose)),
                    naga::Span::default(),
                );
                compose
            }
            naga::TypeInner::Vector {
                size: naga::VectorSize::Quad,
                ..
            } => {
                // vec4 field → use directly
                new_load
            }
            _ => {
                // Fallback: use the loaded value directly
                new_load
            }
        };

        // If the function returns a struct (e.g. FragmentOutput with frag_depth),
        // store the computed color into the output struct and return it.
        // Otherwise return the color value directly (existing behavior).
        let final_value = if let Some(ref result) = ep.function.result {
            if let naga::TypeInner::Struct {
                members: return_members,
                ..
            } = &module.types[result.ty].inner
            {
                let return_ty = result.ty;
                let color_idx = return_members
                    .iter()
                    .position(|m| {
                        matches!(
                            &m.binding,
                            Some(naga::Binding::Location { location: 0, .. })
                        )
                    })
                    .unwrap_or(0);

                let output_var_handle = ep
                    .function
                    .local_variables
                    .iter()
                    .find(|(_, v)| v.ty == return_ty)
                    .map(|(h, _)| h);

                if let Some(output_var_handle) = output_var_handle {
                    let output_var_expr = ep
                            .function
                            .expressions
                            .iter()
                            .find(|(_, e)| {
                                matches!(e, naga::Expression::LocalVariable(v) if *v == output_var_handle)
                            })
                            .map(|(h, _)| h)
                            .expect("must have expression for output variable");

                    let store_access = ep.function.expressions.append(
                        naga::Expression::AccessIndex {
                            base: output_var_expr,
                            index: color_idx as u32,
                        },
                        naga::Span::default(),
                    );
                    new_body.push(
                        naga::Statement::Store {
                            pointer: store_access,
                            value: return_value,
                        },
                        naga::Span::default(),
                    );

                    let output_load = ep.function.expressions.append(
                        naga::Expression::Load {
                            pointer: output_var_expr,
                        },
                        naga::Span::default(),
                    );
                    new_body.push(
                        naga::Statement::Emit(naga::Range::new_from_bounds(
                            output_load,
                            output_load,
                        )),
                        naga::Span::default(),
                    );
                    output_load
                } else {
                    return_value
                }
            } else {
                return_value
            }
        } else {
            return_value
        };

        new_body.push(
            naga::Statement::Return {
                value: Some(final_value),
            },
            naga::Span::default(),
        );
    }

    entry_point!(module, binding_builder.shader_stage)
        .function
        .body = new_body;

    let info =
        naga::valid::Validator::new(ValidationFlags::empty(), naga::valid::Capabilities::all())
            .validate(module)
            .map_err(Box::new)?;
    let output_str = naga::back::wgsl::write_string(module, &info, WriterFlags::all())?;
    log::debug!("Resulting shader code (inject_before_return):\n{output_str}");
    Ok(())
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
