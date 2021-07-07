use structopt::StructOpt;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

mod application;
mod camera_controller;
mod camera_uniforms;
mod custom_event;
mod drop_event;
mod init_wgpu;
mod mesh;
mod pipeline;
mod sphere;
mod vec_to_buffer;
mod vertex_attr;
mod vertex_attr_format;

pub use vertex_attr::VertexAttr;
pub use vertex_attr_format::VertexAttrFormat;

#[cfg(not(target_arch = "wasm32"))]
mod setup_other;
#[cfg(target_arch = "wasm32")]
mod setup_wasm;

use application::Application;
use custom_event::CustomEvent;

type Vector2 = cgmath::Vector2<f32>;
type Vector3 = cgmath::Vector3<f32>;
type Matrix4 = cgmath::Matrix4<f32>;
type Point3 = cgmath::Point3<f32>;

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    load_zdf: Option<std::path::PathBuf>,
}

fn main() {
    let args = Cli::from_args();

    let event_loop = EventLoop::<CustomEvent>::with_user_event();
    let proxy = event_loop.create_proxy();
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title("Visula");
    let window = builder.build(&event_loop).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    {
        setup_other::setup_other(window, proxy);
    }
    #[cfg(target_arch = "wasm32")]
    {
        // TODO should be enough with one proxy
        use std::cell::RefCell;
        use std::rc::Rc;
        let drop_proxy_main = Rc::new(RefCell::new(event_loop.create_proxy()));
        setup_wasm::setup_wasm(window, proxy, drop_proxy_main);
    }

    log::info!("Initializing application");

    let mut application: Option<Application> = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::UserEvent(CustomEvent::Ready(mut app)) => {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(filename) = &args.load_zdf {
                    app.handle_zdf(filename);
                }
                application = Some(app);
            }
            event => match &mut application {
                None => {}
                Some(app) => {
                    app.handle_event(&event, control_flow);
                }
            },
        }
    });
}
