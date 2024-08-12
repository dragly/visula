use std::borrow::BorrowMut;
use visula::initialize_event_loop_and_window_with_config;
use visula::initialize_logger;
use visula::initialize_panic_hook;
use visula::spawn;
use visula::CustomEvent;
use visula::Renderable;
use visula::RunConfig;
use visula::{
    Camera, CameraController, CameraControllerResponse, Expression, InstanceBuffer, LineDelegate,
    Lines, RenderData, RenderingDescriptor,
};
use visula_derive::Instance;

use bytemuck::{Pod, Zeroable};
use std::borrow::Cow;
use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
};

#[repr(C, align(16))]
#[derive(Clone, Copy, Instance, Pod, Zeroable)]
struct LineData {
    position_a: [f32; 3],
    position_b: [f32; 3],
    _padding: [f32; 2],
}

fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> wgpu::TextureView {
    let multisampled_texture_extent = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: config.view_formats[0],
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    };

    device
        .create_texture(multisampled_frame_descriptor)
        .create_view(&wgpu::TextureViewDescriptor::default())
}

async fn run(event_loop: EventLoop<CustomEvent>, window: Window) {
    let size = window.inner_size();
    let mut window_arc = Arc::new(window);

    let instance = wgpu::Instance::default();

    let surface = instance.create_surface(window_arc.clone()).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            // Request an adapter which can render to our surface
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the swapchain.
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let swapchain_format = swapchain_capabilities.formats[0];

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(swapchain_format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 4,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    let mut config = surface
        .get_default_config(&adapter, size.width.max(640), size.height.max(480))
        .expect("Surface isn't supported by the adapter.");

    let surface_view_format = config.format.add_srgb_suffix();
    config.view_formats.push(surface_view_format);

    surface.configure(&device, &config);

    let depth_texture_in = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 4,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    });
    let depth_texture = depth_texture_in.create_view(&wgpu::TextureViewDescriptor::default());
    let multisampled_framebuffer = create_multisampled_framebuffer(&device, &config, 4);

    let line_data = vec![LineData {
        position_a: [-10.0, 0.0, 0.0],
        position_b: [10.0, 0.0, 0.0],
        _padding: [0.0; 2],
    }];
    let line_buffer = InstanceBuffer::<LineData>::new_with_init(&device, &line_data);
    let line = line_buffer.instance();

    let camera = Camera::new(&device);

    let lines = Lines::new(
        &RenderingDescriptor {
            device: &device,
            format: &config.format,
            camera: &camera,
        },
        &LineDelegate {
            start: line.position_a,
            end: line.position_b,
            width: {
                let a: Expression = 1.0.into();
                a + 1.0 + 2.0
            },
            start_color: glam::Vec3::new(1.0, 0.8, 1.0).into(),
            end_color: glam::Vec3::new(1.0, 0.8, 1.0).into(),
        },
    )
    .unwrap();

    let mut camera_controller = CameraController::new(window_arc.borrow_mut());
    event_loop
        .run(move |event, target| {
            // Have the closure take ownership of the resources.
            // `event_loop.run` never returns, therefore we must do this to ensure
            // the resources are properly cleaned up.
            let _ = (&instance, &adapter, &shader, &pipeline_layout);

            camera_controller.update();
            let CameraControllerResponse {
                needs_redraw,
                captured_event,
            } = camera_controller.handle_event(&event);
            if needs_redraw {
                window_arc.borrow_mut().request_redraw();
            }
            if captured_event {
                return;
            }
            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    // Reconfigure the surface with the new size
                    config.width = size.width;
                    config.height = size.height;
                    surface.configure(&device, &config);
                    // On macos the window needs to be redrawn manually after resizing
                    window_arc.borrow_mut().request_redraw();
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    let camera_uniforms =
                        camera_controller.uniforms(config.width as f32 / config.height as f32);
                    camera.update(&camera_uniforms, &queue);
                    let frame = surface
                        .get_current_texture()
                        .expect("Failed to acquire next swap chain texture");
                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &multisampled_framebuffer,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_texture,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    {
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &multisampled_framebuffer,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &depth_texture,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                },
                            ),
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                        rpass.set_pipeline(&render_pipeline);
                        rpass.draw(0..3, 0..1);
                    }

                    let mut render_data = RenderData {
                        view: &view,
                        multisampled_framebuffer: &multisampled_framebuffer,
                        depth_texture: &depth_texture,
                        encoder: &mut encoder,
                        camera: &camera,
                    };
                    lines.render(&mut render_data);

                    queue.submit(Some(encoder.finish()));
                    frame.present();
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => target.exit(),
                _ => {}
            }
        })
        .expect("Failed to run event loop");
}

fn main() {
    let config = RunConfig {
        canvas_name: "canvas".to_owned(),
    };
    initialize_logger();
    initialize_panic_hook();
    let (event_loop, window) = initialize_event_loop_and_window_with_config(config);
    spawn(run(event_loop, window));
}
