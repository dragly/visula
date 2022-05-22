use crate::camera::uniforms::CameraUniforms;
use crate::{Point3, Vector2, Vector3};
use cgmath::prelude::*;

use winit::{
    dpi::PhysicalPosition,
    event::{
        ElementState, MouseButton,
        MouseScrollDelta::{LineDelta, PixelDelta},
        WindowEvent,
    },
};

#[derive(Debug)]
pub struct CameraController {
    left_pressed: bool,
    right_pressed: bool,
    control_pressed: bool,
    previous_postion: Option<PhysicalPosition<f64>>,
    pub distance: f32,
    pub center: Vector3,
    pub forward: Vector3,
    pub up: Vector3,
    pub rotational_speed: f32,
    pub roll_speed: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraController {
    pub fn new() -> CameraController {
        let up = Vector3::unit_y();
        let forward = Vector3::unit_z();
        let right = Vector3::cross(forward, up).normalize();
        let offset_up = up * 0.3;
        let offset_right = -right * 0.4;
        let offset = offset_up + offset_right;
        let axis = Vector3::cross(offset, forward).normalize();
        let rotation = cgmath::Quaternion::from_axis_angle(
            axis,
            cgmath::Rad(
                0.4,
            ),
        );
        let new_forward = (rotation * forward).normalize();
        CameraController {
            left_pressed: false,
            right_pressed: false,
            control_pressed: false,
            forward: new_forward,
            up,
            distance: 200.0,
            center: Vector3::new(0.0, 0.0, 0.0),
            previous_postion: None,
            rotational_speed: 0.005,
            roll_speed: 0.005,
        }
    }

    pub fn update(&mut self) {}

    pub fn handle_event(&mut self, window_event: &WindowEvent) -> bool {
        let mut needs_redraw = false;

        let up = self.up.normalize();
        let forward = self.forward.normalize();
        let right = Vector3::cross(forward, up).normalize();
        let flat_forward = Vector3::cross(up, right).normalize();

        match window_event {
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
                needs_redraw = true;
            }
            WindowEvent::MouseInput { state, button, .. } => match &button {
                MouseButton::Left => match state {
                    ElementState::Pressed => self.left_pressed = true,
                    ElementState::Released => self.left_pressed = false,
                },
                MouseButton::Right => match state {
                    ElementState::Pressed => self.right_pressed = true,
                    ElementState::Released => self.right_pressed = false,
                },
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(previous_postion) = self.previous_postion {
                    let position_diff = Vector2 {
                        x: position.x as f32 - previous_postion.x as f32,
                        y: position.y as f32 - previous_postion.y as f32,
                    };
                    if self.left_pressed {
                        if self.control_pressed {
                            let offset_up = -position_diff.y as f32;
                            let offset_right = position_diff.x as f32;
                            let offset = offset_up + offset_right;
                            let rotation = cgmath::Quaternion::from_axis_angle(
                                forward,
                                cgmath::Rad(self.roll_speed * offset),
                            );
                            self.up = (rotation * self.up).normalize();
                            self.forward = (rotation * self.forward).normalize();
                        } else {
                            if (position_diff.x + position_diff.y).abs() < 0.000001 {
                                self.previous_postion = Some(*position);
                                return false;
                            }
                            let offset_up = up * position_diff.y as f32;
                            let offset_right = -right * position_diff.x as f32;
                            let offset = offset_up + offset_right;
                            let axis = Vector3::cross(offset, forward).normalize();
                            let rotation = cgmath::Quaternion::from_axis_angle(
                                axis,
                                cgmath::Rad(
                                    self.rotational_speed * position_diff.magnitude() as f32,
                                ),
                            );
                            let new_forward = (rotation * self.forward).normalize();
                            if Vector3::dot(up, new_forward).abs() > 0.99 {
                                if position_diff.x.abs() < 0.00001 {
                                    self.previous_postion = Some(*position);
                                    return false;
                                }
                                let offset = offset_right;
                                let axis = Vector3::cross(offset, forward).normalize();
                                let rotation = cgmath::Quaternion::from_axis_angle(
                                    axis,
                                    cgmath::Rad(
                                        self.rotational_speed * (position_diff.x).abs() as f32,
                                    ),
                                );
                                let new_forward = (rotation * self.forward).normalize();
                                self.forward = new_forward;
                                self.previous_postion = Some(*position);
                                return true;
                            }
                            self.forward = new_forward;
                        }
                        needs_redraw = true;
                    }
                    if self.right_pressed {
                        if self.control_pressed {
                            self.center +=
                                up * position_diff.y as f32 - right * position_diff.x as f32;
                        } else {
                            self.center += flat_forward * position_diff.y as f32
                                - right * position_diff.x as f32;
                        }
                        needs_redraw = true;
                    }
                }
                // TODO add previous position separate for left and right and only set it when
                // button down
                self.previous_postion = Some(*position)
            }
            _ => {}
        };

        needs_redraw
    }

    pub fn model_view_projection_matrix(&self, aspect_ratio: f32) -> CameraUniforms {
        create_model_view_projection(aspect_ratio, self)
    }
}

fn create_model_view_projection(
    aspect_ratio: f32,
    CameraController {
        up,
        forward,
        center,
        distance,
        ..
    }: &CameraController,
) -> CameraUniforms {
    let view_vector = forward * *distance;
    let projection_matrix = cgmath::perspective(cgmath::Deg(20f32), aspect_ratio, 10.0, 10000.0);
    let position = Point3::new(
        center.x - view_vector.x,
        center.y - view_vector.y,
        center.z - view_vector.z,
    );
    let view_matrix =
        cgmath::Matrix4::look_at(position, Point3::new(center.x, center.y, center.z), *up);

    let model_view_projection_matrix = OPENGL_TO_WGPU_MATRIX * projection_matrix * view_matrix;

    CameraUniforms {
        view_matrix,
        model_view_projection_matrix,
        center: *center,
        dummy0: 0.0,
        view_vector,
        dummy1: 0.0,
        position: position - Point3::new(0.0, 0.0, 0.0),
        dummy2: 0.0,
        up: *up,
        dummy3: 0.0,
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
