use crate::camera::Camera;
use crate::light::DirectionalLight;
use crate::post_process::PostProcessor;
use crate::rendering_descriptor::RenderingDescriptor;
use crate::simulation::ShadowRenderData;
use crate::{camera::controller::CameraController, simulation::RenderData};
use crate::{CameraControllerResponse, Simulation};
use chrono::{DateTime, Utc};
use egui::{Context, ViewportId};
use egui_wgpu::Renderer;
use egui_wgpu::ScreenDescriptor;
use egui_winit::State;
use winit::window::WindowId;

use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wgpu::{
    BackendOptions, Color, CommandEncoder, CurrentSurfaceTexture, Device, Dx12BackendOptions,
    GlBackendOptions, InstanceDescriptor, SurfaceTexture, TextureFormat, TextureView,
    TextureViewDescriptor,
};
use winit::{event::WindowEvent, window::Window};

#[derive(Debug)]
pub struct Application {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub window: Arc<Window>,
    pub camera_controller: CameraController,
    pub depth_texture: wgpu::TextureView,
    pub multisampled_framebuffer: wgpu::TextureView,
    pub egui_renderer: EguiRenderer,
    pub egui_ctx: egui::Context,
    pub camera: Camera,
    pub light: DirectionalLight,
    pub post_processor: PostProcessor,
    pub start_time: DateTime<Utc>,
    pub sample_count: u32,
    pending_screenshot: Option<PathBuf>,
    surface_supports_copy_src: bool,
}

fn create_egui_context() -> egui::Context {
    pub const IS_DESKTOP: bool = cfg!(any(
        target_os = "freebsd",
        target_os = "linux",
        target_os = "macos",
        target_os = "openbsd",
        target_os = "windows",
    ));

    let egui_ctx = egui::Context::default();
    egui_ctx.set_embed_viewports(!IS_DESKTOP);
    egui_ctx
}

pub struct EguiRenderer {
    pub state: State,
    pub renderer: Renderer,
}

impl Debug for EguiRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EguiRenderer {..} ")?;
        Ok(())
    }
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        window: &Window,
    ) -> EguiRenderer {
        let egui_context = Context::default();
        let egui_state = egui_winit::State::new(
            egui_context,
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        //egui_state.set_pixels_per_point(window.scale_factor() as f32);
        let egui_renderer = egui_wgpu::Renderer::new(
            device,
            output_color_format,
            egui_wgpu::RendererOptions {
                depth_stencil_format: None,
                msaa_samples: 1,
                ..Default::default()
            },
        );

        EguiRenderer {
            state: egui_state,
            renderer: egui_renderer,
        }
    }
}

impl Application {
    pub async fn new(window: Arc<Window>) -> Result<Application, crate::error::Error> {
        let size = window.inner_size();

        #[cfg(target_arch = "wasm32")]
        let backends = wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL;
        #[cfg(not(target_arch = "wasm32"))]
        let backends = wgpu::Backends::from_env().unwrap_or_else(wgpu::Backends::all);

        let dx12_shader_compiler = wgpu::Dx12Compiler::from_env().unwrap_or_default();
        let gles_minor_version = wgpu::Gles3MinorVersion::from_env().unwrap_or_default();

        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends,
            backend_options: BackendOptions {
                gl: GlBackendOptions {
                    gles_minor_version,
                    fence_behavior: wgpu::GlFenceBehavior::default(),
                    debug_fns: wgpu::GlDebugFns::default(),
                },
                dx12: Dx12BackendOptions {
                    shader_compiler: dx12_shader_compiler,
                    ..Default::default()
                },
                noop: wgpu::NoopBackendOptions::default(),
            },
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            display: None,
        });
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|_| crate::error::Error::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::Performance,
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_supports_copy_src = surface_caps.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let mut config = surface
            .get_default_config(&adapter, size.width.max(640), size.height.max(480))
            .ok_or(crate::error::Error::NoSurfaceConfig)?;
        config.present_mode = wgpu::PresentMode::Fifo;
        if surface_supports_copy_src {
            config.usage |= wgpu::TextureUsages::COPY_SRC;
        }
        let surface_view_format = config.format.add_srgb_suffix();
        config.view_formats.push(surface_view_format);
        surface.configure(&device, &config);

        let camera_controller = CameraController::new(&window);

        #[cfg(target_arch = "wasm32")]
        let sample_count = 1;
        #[cfg(not(target_arch = "wasm32"))]
        let sample_count = 4;
        let depth_texture_in = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: None,
            view_formats: &[],
        });
        let depth_texture = depth_texture_in.create_view(&wgpu::TextureViewDescriptor::default());

        let multisampled_framebuffer = Self::create_multisampled_framebuffer(
            &device,
            config.width,
            config.height,
            sample_count,
        );

        let start_time = Utc::now();

        let camera = Camera::new(&device);
        let light = DirectionalLight::new(&device);
        let post_processor = PostProcessor::new(
            &device,
            &queue,
            config.width,
            config.height,
            surface_view_format,
            &camera,
            &depth_texture,
            sample_count,
        );

        let egui_ctx = create_egui_context();
        let egui_renderer = EguiRenderer::new(&device, surface_view_format, &window);

        Ok(Application {
            device,
            queue,
            config,
            camera_controller,
            surface,
            window,
            depth_texture,
            multisampled_framebuffer,
            camera,
            light,
            post_processor,
            egui_renderer,
            egui_ctx,
            start_time,
            sample_count,
            pending_screenshot: None,
            surface_supports_copy_src,
        })
    }

    /// Request that the next rendered frame be saved as a PNG at `path`.
    ///
    /// The capture happens during the next call to [`Application::render`]; the file is on disk
    /// by the time that call returns. Returns `false` if the surface does not support
    /// `COPY_SRC` (in which case the request is dropped).
    pub fn request_screenshot(&mut self, path: impl Into<PathBuf>) -> bool {
        if !self.surface_supports_copy_src {
            log::warn!("screenshot requested but surface does not support COPY_SRC");
            return false;
        }
        self.pending_screenshot = Some(path.into());
        true
    }

    /// Whether [`Application::request_screenshot`] will succeed for this application.
    pub fn supports_screenshot(&self) -> bool {
        self.surface_supports_copy_src
    }

    fn create_multisampled_framebuffer(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        sample_count: u32,
    ) -> wgpu::TextureView {
        let multisampled_texture_extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn window_event(&mut self, window_id: WindowId, event: &WindowEvent) -> bool {
        if window_id != self.window.id() {
            return false;
        }
        let response = self
            .egui_renderer
            .state
            .on_window_event(&self.window, event);
        // Always forward file drop events to the simulation, even if egui consumed them.
        let is_file_drop = matches!(
            event,
            WindowEvent::DroppedFile(_)
                | WindowEvent::HoveredFile(_)
                | WindowEvent::HoveredFileCancelled
        );
        if response.consumed && !is_file_drop {
            return true;
        }
        let egui_ctx = self.egui_renderer.state.egui_ctx().clone();
        let is_pointer_event = matches!(
            event,
            WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::Touch(_)
        );
        let is_keyboard_event = matches!(
            event,
            WindowEvent::KeyboardInput { .. } | WindowEvent::Ime(_)
        );
        if (is_pointer_event && egui_ctx.egui_wants_pointer_input())
            || (is_keyboard_event && egui_ctx.egui_wants_keyboard_input())
        {
            return true;
        }
        let CameraControllerResponse {
            needs_redraw,
            captured_event,
        } = self.camera_controller.window_event(window_id, event);
        if needs_redraw {
            self.window.request_redraw();
        }
        if captured_event {
            return true;
        }
        match event {
            WindowEvent::CloseRequested => println!("{}", crude_profiler::report()),
            WindowEvent::Resized(size) => {
                let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    size: wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: self.sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    label: None,
                    view_formats: &[],
                });
                self.depth_texture =
                    depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.config.width = size.width;
                self.config.height = size.height;
                self.multisampled_framebuffer = Self::create_multisampled_framebuffer(
                    &self.device,
                    size.width,
                    size.height,
                    self.sample_count,
                );
                self.surface.configure(&self.device, &self.config);
                self.post_processor.resize(
                    &self.device,
                    size.width,
                    size.height,
                    &self.depth_texture,
                    &self.camera,
                );
            }
            _ => {}
        }
        false
    }

    pub fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: &winit::event::DeviceEvent,
    ) {
        self.camera_controller.device_event(event);
    }

    pub fn update(&mut self) {
        self.camera_controller.update();
        let camera_uniforms = self
            .camera_controller
            .uniforms(self.config.width as f32, self.config.height as f32);
        self.camera.update(&camera_uniforms, &self.queue);
        self.light.update(&self.queue);
    }

    pub fn next_frame(&self) -> Result<SurfaceTexture, crate::error::Error> {
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) => Ok(frame),
            CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                Ok(frame)
            }
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    CurrentSurfaceTexture::Success(frame)
                    | CurrentSurfaceTexture::Suboptimal(frame) => Ok(frame),
                    other => Err(crate::error::Error::SurfaceTexture(other)),
                }
            }
            other => Err(crate::error::Error::SurfaceTexture(other)),
        }
    }

    pub fn encoder(&self) -> CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }

    pub fn clear(&self, view: &TextureView, encoder: &mut CommandEncoder, clear_color: Color) {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.multisampled_framebuffer,
                resolve_target: Some(view),
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
    }

    pub fn render(&mut self, simulation: &mut impl Simulation) {
        let frame = match self.next_frame() {
            Ok(frame) => frame,
            Err(e) => {
                log::error!("Failed to acquire frame: {e}");
                return;
            }
        };
        let mut encoder = self.encoder();

        let view = frame.texture.create_view(&TextureViewDescriptor {
            format: Some(self.config.view_formats[0]),
            ..wgpu::TextureViewDescriptor::default()
        });

        {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow clear"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.light.shadow_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }
        simulation.render_shadow(&mut ShadowRenderData {
            encoder: &mut encoder,
            shadow_texture: &self.light.shadow_texture_view,
            light: &self.light,
        });

        let msaa = self.sample_count > 1;
        {
            let hdr_view = &self.post_processor.hdr_view;
            let normal_msaa_view = &self.post_processor.normal_msaa_view;
            let normal_resolve_view = &self.post_processor.normal_resolve_view;
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: if msaa {
                            &self.multisampled_framebuffer
                        } else {
                            hdr_view
                        },
                        resolve_target: if msaa { Some(hdr_view) } else { None },
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(simulation.clear_color()),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: if msaa {
                            normal_msaa_view
                        } else {
                            normal_resolve_view
                        },
                        resolve_target: if msaa {
                            Some(normal_resolve_view)
                        } else {
                            None
                        },
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }

        self.post_processor.render_sky(
            &mut encoder,
            &self.queue,
            if msaa {
                &self.multisampled_framebuffer
            } else {
                &self.post_processor.hdr_view
            },
            &self.depth_texture,
            &self.camera,
        );

        simulation.render(&mut RenderData {
            view: &self.post_processor.hdr_view,
            multisampled_framebuffer: if msaa {
                &self.multisampled_framebuffer
            } else {
                &self.post_processor.hdr_view
            },
            depth_texture: &self.depth_texture,
            normal_msaa: if msaa {
                &self.post_processor.normal_msaa_view
            } else {
                &self.post_processor.normal_resolve_view
            },
            normal_resolve: &self.post_processor.normal_resolve_view,
            encoder: &mut encoder,
            camera: &self.camera,
            light: &self.light,
        });

        self.post_processor.render_ssao(&mut encoder, &self.queue);

        self.post_processor
            .render_outline(&mut encoder, &self.queue);

        self.post_processor.render_bloom(&mut encoder, &self.queue);

        self.post_processor
            .render_tonemap(&mut encoder, &self.queue, &view);

        let raw_input = self.egui_renderer.state.take_egui_input(&self.window);
        #[allow(deprecated)]
        let full_output = self.egui_renderer.state.egui_ctx().run(raw_input, |ui| {
            simulation.gui(self, ui);
        });

        self.egui_renderer
            .state
            .handle_platform_output(&self.window, full_output.platform_output);

        let tris = self
            .egui_renderer
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };
        self.egui_renderer.renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        self.egui_renderer.renderer.render(
            &mut render_pass.forget_lifetime(),
            &tris,
            &screen_descriptor,
        );
        for x in &full_output.textures_delta.free {
            self.egui_renderer.renderer.free_texture(x)
        }

        let screenshot_in_flight =
            self.encode_screenshot_copy_if_pending(&mut encoder, &frame.texture);

        self.queue.submit(Some(encoder.finish()));
        frame.present();

        if let Some(pending) = screenshot_in_flight {
            pending.write(&self.device);
        }
    }

    /// If a screenshot has been requested via [`Application::request_screenshot`], encode a
    /// texture-to-buffer copy of `target` into `encoder` and return the in-flight screenshot.
    /// The caller is responsible for submitting `encoder`, presenting the frame, and then
    /// calling [`PendingScreenshot::write`] to finalize the PNG. Returns `None` if no
    /// screenshot was pending.
    pub fn encode_screenshot_copy_if_pending(
        &mut self,
        encoder: &mut CommandEncoder,
        target: &wgpu::Texture,
    ) -> Option<PendingScreenshot> {
        let path = self.pending_screenshot.take()?;
        let width = self.config.width;
        let height = self.config.height;
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padded_bytes_per_row = unpadded_bytes_per_row
            .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visula screenshot buffer"),
            size: (padded_bytes_per_row as u64) * (height as u64),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        Some(PendingScreenshot {
            buffer,
            path,
            padded_bytes_per_row,
            width,
            height,
            format: self.config.format,
        })
    }

    pub fn rendering_descriptor(&self) -> RenderingDescriptor<'_> {
        RenderingDescriptor {
            device: &self.device,
            format: wgpu::TextureFormat::Rgba16Float,
            camera: &self.camera,
            light: &self.light,
            sample_count: self.sample_count,
        }
    }
}

/// Handle to a screenshot whose texture-to-buffer copy has been encoded but not yet finalized.
/// Returned by [`Application::encode_screenshot_copy_if_pending`].
pub struct PendingScreenshot {
    buffer: wgpu::Buffer,
    path: PathBuf,
    padded_bytes_per_row: u32,
    width: u32,
    height: u32,
    format: TextureFormat,
}

impl PendingScreenshot {
    /// Map the buffer (blocking on the device), decode the pixels, and write a PNG to the
    /// path captured when the screenshot was requested. Must be called after the encoder
    /// that produced this `PendingScreenshot` has been submitted.
    pub fn write(self, device: &wgpu::Device) {
        let slice = self.buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| {
            if let Err(e) = result {
                log::error!("failed to map screenshot buffer: {e}");
            }
        });
        if let Err(e) = device.poll(wgpu::PollType::wait_indefinitely()) {
            log::error!("device poll failed while capturing screenshot: {e}");
            return;
        }
        let data = slice.get_mapped_range();
        let result = save_png(
            &data,
            self.padded_bytes_per_row,
            self.width,
            self.height,
            self.format,
            &self.path,
        );
        drop(data);
        self.buffer.unmap();
        match result {
            Ok(()) => log::info!("screenshot saved to {}", self.path.display()),
            Err(e) => log::error!("failed to save screenshot to {}: {e}", self.path.display()),
        }
    }
}

fn save_png(
    data: &[u8],
    padded_bytes_per_row: u32,
    width: u32,
    height: u32,
    format: TextureFormat,
    path: &Path,
) -> Result<(), image::ImageError> {
    let swap_rb = matches!(
        format,
        TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb
    );
    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);
    for row in 0..height as usize {
        let row_start = row * padded_bytes_per_row as usize;
        for px in 0..width as usize {
            let i = row_start + px * 4;
            if swap_rb {
                rgba.push(data[i + 2]);
                rgba.push(data[i + 1]);
                rgba.push(data[i]);
                rgba.push(data[i + 3]);
            } else {
                rgba.extend_from_slice(&data[i..i + 4]);
            }
        }
    }
    image::save_buffer(path, &rgba, width, height, image::ColorType::Rgba8)
}
