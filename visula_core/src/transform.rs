use naga::{Arena, Expression, Handle, Span};

fn transform(
    source_arena: &Arena<Expression>,
    source_handle: Handle<Expression>,
    target_arena: &mut Arena<Expression>,
) -> Handle<Expression> {
    let source_expression = &source_arena[source_handle];
    match source_expression {
        &Expression::Binary { op, left, right } => {
            let target_left = transform(source_arena, left, target_arena);
            let target_right = transform(source_arena, right, target_arena);
            target_arena.append(
                Expression::Binary {
                    op,
                    left: target_left,
                    right: target_right,
                },
                Span::default(),
            )
        }
        &Expression::Literal(ref literal) => {
            target_arena.append(Expression::Literal(literal.clone()), Span::default())
        }
        x => {
            unimplemented!("expression type not supported: {:?}", x)
        }
    }
}

#[cfg(test)]
mod tests {
    use naga::Literal;

    use super::*;

    #[test]
    fn test_transform() {
        let mut source_arena = Arena::new();
        let mut target_arena = Arena::new();
        let left = source_arena.append(Expression::Literal(Literal::F32(1.2)), Span::default());
        let right = source_arena.append(Expression::Literal(Literal::F32(2.4)), Span::default());
        let source_handle = source_arena.append(
            Expression::Binary {
                op: naga::BinaryOperator::Add,
                left,
                right,
            },
            Span::default(),
        );
        let target_handle = transform(&source_arena, source_handle, &mut target_arena);
        dbg!(&source_arena);
        dbg!(&target_arena);
        dbg!(&target_handle);
    }
}
