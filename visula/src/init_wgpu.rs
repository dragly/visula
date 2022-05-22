use crate::application::{Application, DrawMode};
use crate::camera::controller::CameraController;
use crate::camera::uniforms::CameraUniforms;
use crate::custom_event::CustomEvent;
use std::time::Instant;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui::FontDefinitions;
use egui_winit_platform::{Platform, PlatformDescriptor};

use crate::vec_to_buffer::vec_to_buffer;

use std::mem::size_of;
use winit::{event_loop::EventLoopProxy, window::Window};

pub async fn init(proxy: EventLoopProxy<CustomEvent>, window: Window) {
    let size = window.inner_size();

    // TODO remove this when https://github.com/gfx-rs/wgpu/issues/1492 is resolved
    let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    let instance = wgpu::Instance::new(backend);
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                #[cfg(target_arch="wasm32")]
                limits: wgpu::Limits::downlevel_webgl2_defaults(),
                #[cfg(not(target_arch="wasm32"))]
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .unwrap();

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_preferred_format(&adapter).unwrap(),
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    surface.configure(&device, &config);

    let camera_controller = CameraController::new();

    let depth_texture_in = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
    });
    let depth_texture = depth_texture_in.create_view(&wgpu::TextureViewDescriptor::default());

    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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

    let camera_uniform_buffer = vec_to_buffer(
        &device,
        model_view_projection_matrix.as_ref(),
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    );

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_uniform_buffer.as_entire_binding(),
        }],
    });

    let platform = Platform::new(PlatformDescriptor {
        physical_width: size.width as u32,
        physical_height: size.height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });

    let egui_rpass = RenderPass::new(&device, config.format, 1);

    let event_result = proxy.send_event(CustomEvent::Ready(Application {
        camera_uniform_buffer,
        device,
        queue,
        config,
        camera_controller,
        surface,
        window,
        depth_texture,
        draw_mode: DrawMode::default(),
        camera_bind_group_layout,
        camera_bind_group,
        next_buffer_handle: 0,
        platform,
        egui_rpass,
    }));
    if event_result.is_err() {
        println!("ERROR: Could not send event! Is the event loop closed?")
    }
}
