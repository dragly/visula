use js_sys::Uint8Array;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::custom_event::CustomEvent;
use crate::drop_event::DropEvent;
use crate::init_wgpu::init;

#[cfg(target_arch = "wasm32")]
pub fn setup_wasm(
    window: Window,
    proxy: EventLoopProxy<CustomEvent>,
    drop_proxy_main: Rc<RefCell<EventLoopProxy<CustomEvent>>>,
) {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("could not initialize logger");
    log::info!("Start");
    use winit::platform::web::WindowExtWebSys;
    let drag_enter = Closure::wrap(Box::new(|event: &web_sys::Event| {
        event.prevent_default();
        log::info!("Drag enter!");
    }) as Box<dyn FnMut(&web_sys::Event)>);
    let drag_over = Closure::wrap(Box::new(|event: &web_sys::Event| {
        event.prevent_default();
        log::info!("Drag over!");
    }) as Box<dyn FnMut(&web_sys::Event)>);

    let drop_callback = Closure::wrap(Box::new(move |event: &web_sys::Event| {
        event.prevent_default();
        let drag_event_ref: &web_sys::DragEvent = JsCast::unchecked_from_js_ref(event);
        let drag_event = drag_event_ref.clone();
        match drag_event.data_transfer() {
            None => {}
            Some(data_transfer) => match data_transfer.files() {
                None => {}
                Some(files) => {
                    log::info!("Files {:?}", files.length());
                    for i in 0..files.length() {
                        if let Some(file) = files.item(i) {
                            log::info!("Processing file {i}");
                            let drop_proxy_ref = Rc::clone(&drop_proxy_main);
                            let name = file.name();
                            let read_callback = Closure::wrap(Box::new(
                                move |array_buffer: JsValue| {
                                    let array = Uint8Array::new(&array_buffer);
                                    let bytes: Vec<u8> = array.to_vec();
                                    let event_result = (*drop_proxy_ref).borrow_mut().send_event(
                                        CustomEvent::DropEvent(DropEvent {
                                            name: name.clone(),
                                            bytes,
                                        }),
                                    );
                                    log::info!("Sent event");
                                    match event_result {
                                        Ok(_) => {}
                                        Err(_) => {
                                            log::error!("Could not register drop event! Event loop closed?");
                                        }
                                    }
                                },
                            )
                                as Box<dyn FnMut(JsValue)>);
                            let _ = file.array_buffer().then(&read_callback);
                            read_callback.forget();
                        }
                    }
                }
            },
        }
    }) as Box<dyn FnMut(&web_sys::Event)>);

    log::info!("Setting up drag and drop features");
    web_sys::window()
        .and_then(|win| {
            win.set_ondragenter(Some(JsCast::unchecked_from_js_ref(drag_enter.as_ref())));
            win.set_ondragover(Some(JsCast::unchecked_from_js_ref(drag_over.as_ref())));
            win.set_ondrop(Some(JsCast::unchecked_from_js_ref(drop_callback.as_ref())));
            win.document()
        })
        .expect("could not set up window");

    wasm_bindgen_futures::spawn_local(init(proxy, window));

    // From the rustwasm documentation:
    //
    // The instance of `Closure` that we created will invalidate its
    // corresponding JS callback whenever it is dropped, so if we were to
    // normally return from `main` then our registered closure will
    // raise an exception when invoked.
    //
    // Normally we'd store the handle to later get dropped at an appropriate
    // time but for now we want it to be a global handler so we use the
    // `forget` method to drop it without invalidating the closure. Note that
    // this is leaking memory in Rust, so this should be done judiciously!
    drag_enter.forget();
    drag_over.forget();
    drop_callback.forget();
}
