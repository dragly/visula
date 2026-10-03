use naga::back::wgsl::WriterFlags;
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::ShaderStage;

use crate::{inject, BindingBuilder, Expression};

pub fn device() -> wgpu::Device {
    let (device, _queue) = wgpu::Device::noop(&wgpu::DeviceDescriptor::default());
    device
}

fn template(stage: ShaderStage, fields: &[(&str, Expression)]) -> String {
    let members: String = fields
        .iter()
        .zip('a'..='z')
        .map(|((wgsl_type, _), name)| format!("    {name}: {wgsl_type},\n"))
        .collect();
    let entry_point = match stage {
        ShaderStage::Vertex => "@vertex\nfn vs_main() -> @builtin(position) vec4<f32>",
        ShaderStage::Fragment => "@fragment\nfn fs_main() -> @location(0) vec4<f32>",
        _ => panic!("lower_to_wgsl does not support {stage:?}"),
    };
    format!(
        "struct Fields {{\n{members}}}\n\n{entry_point} {{\n    var fields: Fields;\n    let output = fields;\n    return vec4<f32>(0.0);\n}}\n"
    )
}

pub fn lower_to_wgsl(stage: ShaderStage, fields: &[(&str, Expression)]) -> String {
    let source = template(stage, fields);
    let mut module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    let expressions: Vec<Expression> = fields.iter().map(|(_, value)| value.clone()).collect();
    match stage {
        ShaderStage::Vertex => {
            let mut binding_builder = BindingBuilder::new(&module, "vs_main", 0).unwrap();
            inject::inject(&mut module, &mut binding_builder, "fields", &expressions).unwrap();
        }
        _ => {
            let mut binding_builder = BindingBuilder::new(&module, "fs_main", 0).unwrap();
            inject::inject_before_return(&mut module, &mut binding_builder, "fields", &expressions)
                .unwrap();
        }
    }

    let info = Validator::new(ValidationFlags::empty(), Capabilities::all())
        .validate(&module)
        .unwrap();
    let wgsl = naga::back::wgsl::write_string(&module, &info, WriterFlags::all()).unwrap();

    let reparsed = naga::front::wgsl::parse_str(&wgsl)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&wgsl)));
    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&reparsed)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&wgsl)));
    wgsl
}
