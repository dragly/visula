pub trait NagaType {
    fn naga_type() -> naga::Type;
}

macro_rules! add_naga_type {
    ($input:ty, $output:expr) => {
        impl NagaType for $input {
            fn naga_type() -> naga::Type {
                $output
            }
        }
    };
}

add_naga_type! {
   f32, naga::Type {
        name: None,
        inner: naga::TypeInner::Scalar {
            kind: naga::ScalarKind::Float,
            width: 4,
        },
    }
}

macro_rules! add_naga_float_vector {
    ($size:expr, $vector_size:expr) => {
        add_naga_type! {
            [f32; $size], naga::Type {
                name: None,
                inner: naga::TypeInner::Vector {
                    kind: naga::ScalarKind::Float,
                    width: 4,
                    size: $vector_size,
                },
            }
        }
    };
}

add_naga_float_vector! {2, naga::VectorSize::Bi}
add_naga_float_vector! {3, naga::VectorSize::Tri}
add_naga_float_vector! {4, naga::VectorSize::Quad}
