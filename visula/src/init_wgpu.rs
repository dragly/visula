use crate::{application::Application, camera::Camera};
use crate::camera::controller::CameraController;

use crate::custom_event::CustomEvent;
use egui::FontDefinitions;
use egui_wgpu_backend::RenderPass;
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::InstanceDescriptor;




use winit::{event_loop::EventLoopProxy, window::Window};

pub async fn init(proxy: EventLoopProxy<CustomEvent>, window: Window) {
    let size = window.inner_size();

    // TODO remove this when https://github.com/gfx-rs/wgpu/issues/1492 is resolved
    let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

    let instance = wgpu::Instance::new(InstanceDescriptor {
        backends,
        dx12_shader_compiler,
    });
    let surface = unsafe { instance.create_surface(&window).unwrap() };
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
                #[cfg(target_arch = "wasm32")]
                limits: wgpu::Limits::downlevel_webgl2_defaults(),
                #[cfg(not(target_arch = "wasm32"))]
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .unwrap();

    let mut config = surface
        .get_default_config(&adapter, size.width, size.height)
        .expect("Surface isn't supported by the adapter.");
    let surface_view_format = config.format.add_srgb_suffix();
    config.view_formats.push(surface_view_format);
    surface.configure(&device, &config);

    let camera_controller = CameraController::new(&window);

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
        view_formats: &[],
    });
    let depth_texture = depth_texture_in.create_view(&wgpu::TextureViewDescriptor::default());

    let platform = Platform::new(PlatformDescriptor {
        physical_width: size.width,
        physical_height: size.height,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });

    let egui_rpass = RenderPass::new(&device, config.format, 1);

    let camera = Camera::new(&device);

    let event_result = proxy.send_event(CustomEvent::Ready(Box::new(Application {
        device,
        queue,
        config,
        camera_controller,
        surface,
        window,
        depth_texture,
        camera,
        next_buffer_handle: 0,
        platform,
        egui_rpass,
    })));
    if event_result.is_err() {
        println!("ERROR: Could not send event! Is the event loop closed?")
    }
}
