use std::f32::consts::PI;

use crate::camera::uniforms::CameraUniforms;
use glam::{Mat4, Quat, Vec2, Vec3};

use winit::event::{
    DeviceEvent, ElementState, MouseButton,
    MouseScrollDelta::{LineDelta, PixelDelta},
    WindowEvent,
};
use winit::window::{Window, WindowId};

#[derive(Copy, Clone, Debug, PartialEq)]
enum State {
    Released,
    PressedWaiting,
    Moving,
}

#[derive(Debug)]
pub struct CameraController {
    left_pressed: bool,
    right_pressed: bool,
    control_pressed: bool,
    pub enabled: bool,
    pub distance: f32,
    pub center: Vec3,
    pub forward: Vec3,
    pub true_up: Vec3,
    pub up: Vec3,
    pub rotational_speed: f32,
    pub roll_speed: f32,
    state: State,
    window_id: WindowId,
}

#[derive(Clone, Debug)]
pub struct CameraControllerResponse {
    pub needs_redraw: bool,
    pub captured_event: bool,
}

impl CameraController {
    pub fn new(window: &Window) -> CameraController {
        let up = Vec3::Y;
        let forward = Vec3::Z;
        let right = Vec3::cross(forward, up).normalize();
        let offset_up = up;
        let _offset_right = right;
        let offset = offset_up;
        let axis = Vec3::cross(offset, forward).normalize();
        let rotation = Quat::from_axis_angle(axis, 1.0);
        let new_forward = (rotation * forward).normalize();
        let scale_factor = window.scale_factor() as f32;
        let window_id = window.id();
        CameraController {
            enabled: true,
            left_pressed: false,
            right_pressed: false,
            control_pressed: false,
            forward: new_forward,
            true_up: up,
            up,
            distance: 100.0,
            center: Vec3::new(0.0, 0.0, 0.0),
            rotational_speed: 0.005 / scale_factor,
            roll_speed: 0.005 / scale_factor,
            state: State::Released,
            window_id,
        }
    }

    pub fn update(&mut self) {}

    pub fn device_event(&mut self, event: &DeviceEvent) -> CameraControllerResponse {
        let mut response = CameraControllerResponse {
            needs_redraw: false,
            captured_event: false,
        };
        if !self.enabled {
            return response;
        }
        let up = self.up.normalize();
        let forward = self.forward.normalize();
        let right = Vec3::cross(forward, up).normalize();
        let flat_forward = Vec3::cross(up, right).normalize();
        if let DeviceEvent::MouseMotion { delta, .. } = event {
            let position_diff = Vec2 {
                x: delta.0 as f32,
                y: delta.1 as f32,
            };
            if self.left_pressed {
                if self.control_pressed {
                    let offset_up = -position_diff.y;
                    let offset_right = position_diff.x;
                    let offset = offset_up + offset_right;
                    let rotation = Quat::from_axis_angle(forward, self.roll_speed * offset);
                    self.up = (rotation * self.up).normalize();
                    self.true_up = (rotation * self.true_up).normalize();
                    self.forward = (rotation * self.forward).normalize();
                } else {
                    if (position_diff.x + position_diff.y).abs() < 0.000001 {
                        return CameraControllerResponse {
                            needs_redraw: false,
                            captured_event: false,
                        };
                    }
                    let rotation_x = Quat::from_axis_angle(
                        self.true_up,
                        -self.rotational_speed * position_diff.x,
                    );
                    let rotation_y =
                        Quat::from_axis_angle(right, -self.rotational_speed * position_diff.y);
                    self.forward = (rotation_x * rotation_y * self.forward).normalize();
                    self.up = (rotation_x * rotation_y * self.up).normalize();
                }
                response.needs_redraw = true;
                response.captured_event = true;
                self.state = State::Moving;
            }
            if self.right_pressed {
                if self.control_pressed {
                    self.center += up * position_diff.y - right * position_diff.x;
                } else {
                    self.center += flat_forward * position_diff.y - right * position_diff.x;
                }
                response.needs_redraw = true;
                response.captured_event = true;
            }
        }
        response
    }

    pub fn window_event(
        &mut self,
        window_id: WindowId,
        event: &WindowEvent,
    ) -> CameraControllerResponse {
        let mut response = CameraControllerResponse {
            needs_redraw: false,
            captured_event: false,
        };
        if !self.enabled {
            return response;
        }

        if window_id != self.window_id {
            return response;
        }
        match event {
            WindowEvent::ModifiersChanged(state) => {
                self.control_pressed = state
                    .state()
                    .contains(winit::keyboard::ModifiersState::CONTROL);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let diff = match delta {
                    LineDelta(_x, y) => *y,
                    PixelDelta(delta) => 0.04 * delta.y as f32,
                };
                let factor = 1.0 + 0.1 * diff.abs();
                if diff > 0.0 {
                    self.distance /= factor;
                } else {
                    self.distance *= factor;
                }
                response.needs_redraw = true;
                response.captured_event = true;
            }
            WindowEvent::MouseInput { state, button, .. } => match &button {
                MouseButton::Left => match state {
                    ElementState::Pressed => {
                        self.left_pressed = true;
                        self.state = State::PressedWaiting;
                    }
                    ElementState::Released => {
                        self.left_pressed = false;
                        response.captured_event = self.state == State::Moving;
                        self.state = State::Released;
                    }
                },
                MouseButton::Right => match state {
                    ElementState::Pressed => {
                        self.right_pressed = true;
                        self.state = State::PressedWaiting;
                    }
                    ElementState::Released => {
                        self.right_pressed = false;
                        response.captured_event = self.state == State::Moving;
                        self.state = State::Released;
                    }
                },
                _ => {}
            },
            _ => {}
        }
        response
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.center, self.up)
    }

    pub fn position(&self) -> Vec3 {
        let view_vector = self.forward * self.distance;
        self.center - view_vector
    }

    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        Mat4::perspective_rh(40f32 / 180.0 * PI, aspect_ratio, 10.0, 10000.0)
    }

    pub fn active(&self) -> bool {
        self.state != State::Released
    }

    pub fn uniforms(&self, aspect_ratio: f32) -> CameraUniforms {
        let view_matrix = self.view_matrix();

        let model_view_projection_matrix = self.projection_matrix(aspect_ratio) * view_matrix;

        CameraUniforms {
            view_matrix,
            model_view_projection_matrix,
            center: self.center,
            dummy0: 0.0,
            view_vector: self.forward * self.distance,
            dummy1: 0.0,
            position: self.position() - Vec3::ZERO,
            dummy2: 0.0,
            up: self.up,
            dummy3: 0.0,
        }
    }
}
