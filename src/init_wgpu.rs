use crate::application::Application;
use crate::camera_controller::CameraController;
use crate::camera_uniforms::CameraUniforms;
use crate::custom_event::CustomEvent;
use crate::pipeline::Pipeline;
use crate::sphere::Sphere;
use crate::vec_to_buffer::vec_to_buffer;
use crate::Point3;

use bytemuck;
use bytemuck::{Pod, Zeroable};
use std::mem::size_of;
use winit::{event_loop::EventLoopProxy, window::Window};

pub async fn init(
    proxy: EventLoopProxy<CustomEvent>,
    window: Window,
    swapchain_format: wgpu::TextureFormat,
) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new();
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
            wgpu::BackendBit::PRIMARY,
        )
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                limits: wgpu::Limits::default(),
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
            },
            None,
        )
        .await
        .unwrap();

    let sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    let swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let points = create_point_pipeline(&device, swapchain_format).unwrap();
    let camera_controller = CameraController::new();

    let depth_texture_in = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        label: None,
    });
    let depth_texture = depth_texture_in.create_default_view();

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
    }));
    match event_result {
        Err(_) => println!("ERROR: Could not send event! Is the event loop closed?"),
        _ => {}
    }
}

fn create_point_pipeline(
    device: &wgpu::Device,
    swapchain_format: wgpu::TextureFormat,
) -> Result<Pipeline, Box<dyn std::error::Error>> {
    let vertex_size = size_of::<Vertex>();
    let (vertex_data, index_data) = create_vertices();
    let index_count = index_data.len();

    let vertex_buffer = device.create_buffer_with_data(
        bytemuck::cast_slice(&vertex_data),
        wgpu::BufferUsage::VERTEX,
    );

    let instance_data = vec![Sphere {
        position: Point3::new(0.0, 0.0, 0.0),
        radius: 1.0,
        color: Point3::new(0.2, 0.3, 0.4),
    }];

    let instance_count = instance_data.len();

    let instance_buffer = device.create_buffer_with_data(
        bytemuck::cast_slice(&instance_data),
        wgpu::BufferUsage::VERTEX,
    );

    let index_buffer =
        device.create_buffer_with_data(bytemuck::cast_slice(&index_data), wgpu::BufferUsage::INDEX);

    let vs_bytes = include_bytes!("shader.vert.spv");
    let fs_bytes = include_bytes!("shader.frag.spv");

    let vs_module = device
        .create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs_bytes[..])).unwrap());
    let fs_module = device
        .create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs_bytes[..])).unwrap());

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        bindings: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
            ty: wgpu::BindingType::UniformBuffer { dynamic: false },
        }],
    });

    // TODO get the definition of the size of the camera uniforms into one place somehow
    let model_view_projection_matrix = [0.0; size_of::<CameraUniforms>() / size_of::<f32>()];

    let uniform_buffer = vec_to_buffer(
        &device,
        &model_view_projection_matrix.to_vec(),
        wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        bindings: &[wgpu::Binding {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(uniform_buffer.slice(..)),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        }),
        rasterization_state: Some(wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: swapchain_format,
            color_blend: wgpu::BlendDescriptor::REPLACE,
            alpha_blend: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWrite::ALL,
        }],
        depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
        }),
        vertex_state: wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[
                wgpu::VertexBufferDescriptor {
                    stride: vertex_size as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[wgpu::VertexAttributeDescriptor {
                        format: wgpu::VertexFormat::Float4,
                        offset: 0,
                        shader_location: 0,
                    }],
                },
                wgpu::VertexBufferDescriptor {
                    stride: size_of::<Sphere>() as wgpu::BufferAddress,
                    step_mode: wgpu::InputStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float3,
                            offset: 0,
                            shader_location: 1,
                        },
                        // TODO create a macro for this
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float,
                            offset: 3 * 4,
                            shader_location: 2,
                        },
                        wgpu::VertexAttributeDescriptor {
                            format: wgpu::VertexFormat::Float3,
                            offset: 3 * 4 + 4,
                            shader_location: 3,
                        },
                    ],
                },
            ],
        },
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
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
