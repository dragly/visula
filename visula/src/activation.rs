use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
use winit::window::Window;

#[derive(Debug, Default)]
pub(crate) struct Activation {
    pub(crate) active: bool,
    cursor: Option<PhysicalPosition<f64>>,
    press: Option<PhysicalPosition<f64>>,
}

impl Activation {
    pub(crate) fn is_tap(&mut self, event: &WindowEvent, slop: f64) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Some(*position);
                self.forget_press_beyond(*position, slop);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.press = self.cursor,
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => return self.press.take().is_some(),
            WindowEvent::Touch(touch) => match touch.phase {
                TouchPhase::Started => self.press = Some(touch.location),
                TouchPhase::Moved => self.forget_press_beyond(touch.location, slop),
                TouchPhase::Ended => return self.press.take().is_some(),
                TouchPhase::Cancelled => self.press = None,
            },
            _ => {}
        }
        false
    }

    fn forget_press_beyond(&mut self, position: PhysicalPosition<f64>, slop: f64) {
        self.press = self
            .press
            .filter(|press| (position.x - press.x).hypot(position.y - press.y) <= slop);
    }
}

pub(crate) fn show_hint(context: &egui::Context) {
    egui::Area::new(egui::Id::new("visula_activation_hint"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -16.0])
        .interactable(false)
        .show(context, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label("Click or tap to interact");
            });
        });
}

pub(crate) fn show_done_button(context: &egui::Context) -> bool {
    egui::Area::new(egui::Id::new("visula_activation_done"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_TOP, [-8.0, 8.0])
        .show(context, |ui| ui.button("Done").clicked())
        .inner
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn capture_page_input(window: &Window, capture: bool) {
    use winit::platform::web::WindowExtWebSys;
    window.set_prevent_default(capture);
    let Some(canvas) = window.canvas() else {
        return;
    };
    let style = canvas.style();
    let result = if capture {
        style.set_property("touch-action", "none")
    } else {
        style.remove_property("touch-action").map(|_| ())
    };
    if let Err(error) = result {
        log::warn!("Could not set touch-action on the canvas: {error:?}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn capture_page_input(_window: &Window, _capture: bool) {}
