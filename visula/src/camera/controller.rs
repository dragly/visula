use crate::camera::uniforms::CameraUniforms;
use crate::{Matrix4, Point3, Vector2, Vector3};

use cgmath::prelude::*;

use winit::event::{
    DeviceEvent, ElementState, Event, MouseButton,
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
    pub center: Vector3,
    pub forward: Vector3,
    pub up: Vector3,
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
        let up = Vector3::unit_y();
        let forward = Vector3::unit_z();
        let right = Vector3::cross(forward, up).normalize();
        let offset_up = up;
        let _offset_right = right;
        let offset = offset_up;
        let axis = Vector3::cross(offset, forward).normalize();
        let rotation = cgmath::Quaternion::from_axis_angle(axis, cgmath::Rad(1.0));
        let new_forward = (rotation * forward).normalize();
        let scale_factor = window.scale_factor() as f32;
        let window_id = window.id();
        CameraController {
            enabled: true,
            left_pressed: false,
            right_pressed: false,
            control_pressed: false,
            forward: new_forward,
            up,
            distance: 100.0,
            center: Vector3::new(0.0, 0.0, 0.0),
            rotational_speed: 0.005 / scale_factor,
            roll_speed: 0.005 / scale_factor,
            state: State::Released,
            window_id,
        }
    }

    pub fn update(&mut self) {}

    pub fn handle_event<T>(&mut self, event: &Event<T>) -> CameraControllerResponse {
        let mut response = CameraControllerResponse {
            needs_redraw: false,
            captured_event: false,
        };
        if !self.enabled {
            return response;
        }

        let up = self.up.normalize();
        let forward = self.forward.normalize();
        let right = Vector3::cross(forward, up).normalize();
        let flat_forward = Vector3::cross(up, right).normalize();

        match event {
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta, .. },
                ..
            } => {
                let position_diff = Vector2 {
                    x: delta.0 as f32,
                    y: delta.1 as f32,
                };
                if self.left_pressed {
                    if self.control_pressed {
                        let offset_up = -position_diff.y;
                        let offset_right = position_diff.x;
                        let offset = offset_up + offset_right;
                        let rotation = cgmath::Quaternion::from_axis_angle(
                            forward,
                            cgmath::Rad(self.roll_speed * offset),
                        );
                        self.up = (rotation * self.up).normalize();
                        self.forward = (rotation * self.forward).normalize();
                    } else {
                        if (position_diff.x + position_diff.y).abs() < 0.000001 {
                            return CameraControllerResponse {
                                needs_redraw: false,
                                captured_event: false,
                            };
                        }
                        let offset_up = up * position_diff.y;
                        let offset_right = -right * position_diff.x;
                        let offset = offset_up + offset_right;
                        let axis = Vector3::cross(offset, forward).normalize();
                        let rotation = cgmath::Quaternion::from_axis_angle(
                            axis,
                            cgmath::Rad(self.rotational_speed * position_diff.magnitude()),
                        );
                        let new_forward = (rotation * self.forward).normalize();
                        if Vector3::dot(up, new_forward).abs() > 0.99 {
                            if position_diff.x.abs() < 0.00001 {
                                return CameraControllerResponse {
                                    needs_redraw: false,
                                    captured_event: false,
                                };
                            }
                            let offset = offset_right;
                            let axis = Vector3::cross(offset, forward).normalize();
                            let rotation = cgmath::Quaternion::from_axis_angle(
                                axis,
                                cgmath::Rad(self.rotational_speed * (position_diff.x).abs()),
                            );
                            let new_forward = (rotation * self.forward).normalize();
                            self.forward = new_forward;
                            self.state = State::Moving;
                            return CameraControllerResponse {
                                needs_redraw: true,
                                captured_event: true,
                            };
                        }
                        self.forward = new_forward;
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
            Event::WindowEvent {
                event: window_event,
                window_id,
            } if *window_id == self.window_id => match window_event {
                WindowEvent::ModifiersChanged(state) => {
                    self.control_pressed = state.contains(winit::event::ModifiersState::CTRL);
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
            },
            _ => {}
        }

        response
    }

    pub fn view_matrix(&self) -> Matrix4 {
        cgmath::Matrix4::look_at(
            self.position(),
            Point3::new(self.center.x, self.center.y, self.center.z),
            self.up,
        )
    }

    pub fn position(&self) -> Point3 {
        let view_vector = self.forward * self.distance;
        Point3::new(
            self.center.x - view_vector.x,
            self.center.y - view_vector.y,
            self.center.z - view_vector.z,
        )
    }

    pub fn projection_matrix(&self, aspect_ratio: f32) -> Matrix4 {
        OPENGL_TO_WGPU_MATRIX * cgmath::perspective(cgmath::Deg(40f32), aspect_ratio, 10.0, 10000.0)
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
            position: self.position() - Point3::new(0.0, 0.0, 0.0),
            dummy2: 0.0,
            up: self.up,
            dummy3: 0.0,
        }
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
