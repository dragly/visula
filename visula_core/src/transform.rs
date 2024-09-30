use std::collections::HashMap;

use naga::{Arena, Expression, Handle, Span};

fn transform(
    source_arena: &Arena<Expression>,
    source_handle: Handle<Expression>,
    target_arena: &mut Arena<Expression>,
    handle_map: &mut HashMap<Handle<Expression>, Handle<Expression>>,
) -> Handle<Expression> {
    if let Some(target_handle) = handle_map.get(&source_handle) {
        return target_handle.clone();
    }
    let source_expression = &source_arena[source_handle];
    let target_expression = match source_expression {
        &Expression::Binary { op, left, right } => {
            let target_left = transform(source_arena, left, target_arena, handle_map);
            let target_right = transform(source_arena, right, target_arena, handle_map);
            Expression::Binary {
                op,
                left: target_left,
                right: target_right,
            }
        }
        &Expression::FunctionArgument(x) => Expression::FunctionArgument(x),
        &Expression::AccessIndex { base, index } => {
            let target_base = transform(source_arena, base, target_arena, handle_map);
            Expression::AccessIndex {
                base: target_base,
                index,
            }
        }
        &Expression::GlobalVariable(v) => Expression::GlobalVariable(v),
        &Expression::Load { pointer } => {
            let target_pointer = transform(source_arena, pointer, target_arena, handle_map);
            Expression::Load {
                pointer: target_pointer,
            }
        }
        &Expression::Swizzle {
            size,
            vector,
            pattern,
        } => {
            let target_vector = transform(source_arena, vector, target_arena, handle_map);
            Expression::Swizzle {
                size,
                vector: target_vector,
                pattern,
            }
        }
        &Expression::Math {
            fun,
            arg,
            arg1,
            arg2,
            arg3,
        } => {
            let target_arg = transform(source_arena, arg, target_arena, handle_map);
            let target_arg1 =
                arg1.map(|arg| transform(source_arena, arg, target_arena, handle_map));
            let target_arg2 =
                arg2.map(|arg| transform(source_arena, arg, target_arena, handle_map));
            let target_arg3 =
                arg3.map(|arg| transform(source_arena, arg, target_arena, handle_map));
            Expression::Math {
                fun,
                arg: target_arg,
                arg1: target_arg1,
                arg2: target_arg2,
                arg3: target_arg3,
            }
        }
        &Expression::Unary { op, expr } => {
            let target_expr = transform(source_arena, expr, target_arena, handle_map);
            Expression::Unary {
                op,
                expr: target_expr,
            }
        }
        &Expression::Literal(ref literal) => Expression::Literal(literal.clone()),
        Expression::Compose { ty, components } => {
            let target_components = components
                .iter()
                .map(|&component| transform(source_arena, component, target_arena, handle_map))
                .collect();
            Expression::Compose {
                ty: ty.clone(),
                components: target_components,
            }
        }
        &Expression::LocalVariable(v) => Expression::LocalVariable(v),
        x => {
            unimplemented!("expression type not supported: {:?}", x)
        }
    };
    let target_handle = target_arena.append(target_expression, Span::default());
    handle_map.insert(source_handle, target_handle);
    target_handle
}

#[cfg(test)]
mod tests {
    use naga::Literal;

    use super::*;

    #[test]
    fn test_transform() {
        let mut module = naga::front::wgsl::parse_str(include_str!("test.wgsl")).unwrap();
        let source_arena = &module.entry_points[1].function.expressions;
        let mut target_arena = Arena::new();
        let mut handle_map = HashMap::new();
        for (source_handle, _source_expression) in source_arena.iter() {
            let target_handle = transform(
                &source_arena,
                source_handle,
                &mut target_arena,
                &mut handle_map,
            );
        }
        dbg!(&source_arena);
        dbg!(&target_arena);
    }
}
