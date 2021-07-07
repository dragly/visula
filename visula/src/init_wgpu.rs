use crate::application::{Application, DrawMode};
use crate::camera_controller::CameraController;
use crate::camera_uniforms::CameraUniforms;
use crate::custom_event::CustomEvent;
use crate::mesh::MeshVertexAttributes;
use crate::pipeline::Pipeline;
use crate::sphere::Sphere;
use crate::vec_to_buffer::vec_to_buffer;
use crate::vertex_attr::VertexAttr;

use wgpu::util::DeviceExt;

use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use winit::{event_loop::EventLoopProxy, window::Window};

pub async fn init(proxy: EventLoopProxy<CustomEvent>, window: Window) {
    let size = window.inner_size();

    // TODO remove this when https://github.com/gfx-rs/wgpu/issues/1492 is resolved
    #[cfg(target_arch = "wasm32")]
    let instance = wgpu::Instance::new(wgpu::BackendBit::all());
    #[cfg(not(target_arch = "wasm32"))]
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .unwrap();

    let sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let points = create_point_pipeline(&sc_desc, &device).unwrap();
    let camera_controller = CameraController::new();

    let depth_texture_in = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        label: None,
    });
    let depth_texture = depth_texture_in.create_view(&wgpu::TextureViewDescriptor::default());

    let event_result = proxy.send_event(CustomEvent::Ready(Application {
        device,
        queue,
        sc_desc,
        swap_chain,
        camera_controller,
        surface,
        points,
        window,
        depth_texture,
        draw_mode: DrawMode::default(),
    }));
    if event_result.is_err() {
        println!("ERROR: Could not send event! Is the event loop closed?")
    }
}

fn create_point_pipeline(
    sc_desc: &wgpu::SwapChainDescriptor,
    device: &wgpu::Device,
) -> Result<Pipeline, Box<dyn std::error::Error>> {
    let vertex_size = size_of::<Vertex>();
    let (vertex_data, index_data) = create_vertices();
    let index_count = index_data.len();

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex buffer"),
        contents: bytemuck::cast_slice(&vertex_data),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let instance_data = vec![Sphere {
        position: [0.0, 0.0, 0.0],
        radius: 1.0,
        color: [0.2, 0.3, 0.4],
    }];

    let instance_count = instance_data.len();

    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance buffer"),
        contents: bytemuck::cast_slice(&instance_data),
        usage: wgpu::BufferUsage::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index buffer"),
        contents: bytemuck::cast_slice(&index_data),
        usage: wgpu::BufferUsage::INDEX,
    });

    let mut flags = wgpu::ShaderFlags::VALIDATION;
    flags |= wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION;
    let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shader.wgsl"))),
        flags: wgpu::ShaderFlags::all(),
    });

    let vertex_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: vertex_size as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        }],
    };
    let instance_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: size_of::<Sphere>() as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Instance,
        attributes: &Sphere::attributes(1),
    };

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size_of::<CameraUniforms>() as u64),
            },
            count: None,
        }],
    });

    // TODO get the definition of the size of the camera uniforms into one place somehow
    let model_view_projection_matrix = [0.0; size_of::<CameraUniforms>() / size_of::<f32>()];

    let uniform_buffer = vec_to_buffer(
        device,
        &model_view_projection_matrix.to_vec(),
        wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pipeline"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[vertex_buffer_layout, instance_buffer_layout],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[sc_desc.format.into()],
        }),
        primitive: wgpu::PrimitiveState {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
    });

    let mesh_vertex_size = size_of::<MeshVertexAttributes>();
    let mesh_shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("mesh.wgsl"))),
        flags: wgpu::ShaderFlags::all(),
    });
    let mesh_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mesh Bind Group Layout"),
            entries: &[
                // Regular uniform variables like view/projection.
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<CameraUniforms>() as _),
                    },
                    count: None,
                },
            ],
        });
    let mesh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &mesh_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
        label: Some("Mesh Normal Bind Group"),
    });

    let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh pipeline layout"),
        bind_group_layouts: &[&mesh_bind_group_layout],
        push_constant_ranges: &[],
    });
    let mesh_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: mesh_vertex_size as wgpu::BufferAddress,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4],
    };
    let mesh_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Mesh pipeline"),
        layout: Some(&mesh_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &mesh_shader_module,
            entry_point: "vs_main",
            buffers: &[mesh_buffer_layout],
        },
        fragment: Some(wgpu::FragmentState {
            module: &mesh_shader_module,
            entry_point: "fs_main",
            targets: &[sc_desc.format.into()],
        }),
        primitive: wgpu::PrimitiveState {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
    });

    let mesh_vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance buffer"),
        contents: bytemuck::cast_slice(&Vec::<MeshVertexAttributes>::new()),
        usage: wgpu::BufferUsage::VERTEX,
    });

    Ok(Pipeline {
        render_pipeline,
        bind_group,
        vertex_buffer,
        index_buffer,
        instance_buffer,
        instance_count,
        index_count,
        uniform_buffer,

        mesh_render_pipeline,
        mesh_vertex_buf,
        mesh_vertex_count: 0,
        mesh_bind_group,
    })
}

fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    let vertex_data = [
        vertex([-1, -1, -1], [0, 0]),
        vertex([1, -1, -1], [1, 0]),
        vertex([1, 1, -1], [1, 1]),
        vertex([-1, 1, -1], [0, 1]),
    ];

    let index_data: &[u16] = &[0, 1, 2, 2, 3, 0];

    (vertex_data.to_vec(), index_data.to_vec())
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    _pos: [f32; 3],
    _tex_coord: [f32; 2],
}

unsafe impl Pod for Vertex {}
unsafe impl Zeroable for Vertex {}

fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        _pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
        _tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}
