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

macro_rules! add_naga_float_vector {
    ($size:expr, $vector_size:expr) => {
        add_naga_type! {
            [f32; $size], naga::Type {
                name: None,
                inner: naga::TypeInner::Vector {
                    scalar: naga::Scalar {
                        kind: naga::ScalarKind::Float,
                        width: 4,
                    },
                    size: $vector_size,
                },
            }
        }
    };
}

add_naga_type! {
   f32, naga::Type {
        name: None,
        inner: naga::TypeInner::Scalar(
            naga::Scalar {
                kind: naga::ScalarKind::Float,
                width: 4,
            }
        ),
    }
}

add_naga_type! {
   i32, naga::Type {
        name: None,
        inner: naga::TypeInner::Scalar(
            naga::Scalar{
                kind: naga::ScalarKind::Sint,
                width: 4,
            }
        ),
    }
}

add_naga_float_vector! {2, naga::VectorSize::Bi}
add_naga_float_vector! {3, naga::VectorSize::Tri}
add_naga_float_vector! {4, naga::VectorSize::Quad}

macro_rules! add_naga_glam_vector {
    ($glam_type:ty, $vector_size:expr) => {
        add_naga_type! {
            $glam_type, naga::Type {
                name: None,
                inner: naga::TypeInner::Vector {
                    scalar: naga::Scalar {
                        kind: naga::ScalarKind::Float,
                        width: 4,
                    },
                    size: $vector_size,
                },
            }
        }
    };
}

add_naga_glam_vector! {glam::Vec2, naga::VectorSize::Bi}
add_naga_glam_vector! {glam::Vec3, naga::VectorSize::Tri}
add_naga_glam_vector! {glam::Vec4, naga::VectorSize::Quad}
add_naga_glam_vector! {glam::Quat, naga::VectorSize::Quad}

macro_rules! add_naga_glam_matrix {
    ($glam_type:ty, $columns:expr, $rows:expr) => {
        add_naga_type! {
            $glam_type, naga::Type {
                name: None,
                inner: naga::TypeInner::Matrix {
                    columns: $columns,
                    rows: $rows,
                    scalar: naga::Scalar {
                        kind: naga::ScalarKind::Float,
                        width: 4,
                    },
                },
            }
        }
    };
}

add_naga_glam_matrix! {glam::Mat2, naga::VectorSize::Bi, naga::VectorSize::Bi}
add_naga_glam_matrix! {glam::Mat3, naga::VectorSize::Tri, naga::VectorSize::Tri}
add_naga_glam_matrix! {glam::Mat4, naga::VectorSize::Quad, naga::VectorSize::Quad}
