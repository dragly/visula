use crate::camera_uniforms::CameraUniforms;
use crate::{Point3, Vector3};

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, WindowEvent},
};

pub struct CameraController {
    left_pressed: bool,
    right_pressed: bool,
    previous_postion: Option<PhysicalPosition<f64>>,
    yaw: f32,
    pitch: f32,
    distance: f32,
    center: Vector3,
}

impl CameraController {
    pub fn new() -> CameraController {
        CameraController {
            left_pressed: false,
            right_pressed: false,
            yaw: 0.5,
            pitch: 0.5,
            distance: 3.0,
            center: Vector3::new(0.0, 0.0, 0.0),
            previous_postion: None,
        }
    }

    pub fn handle_event(&mut self, window_event: &WindowEvent) -> bool {
        let mut needs_redraw = false;

        match window_event {
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
                    let position_diff = PhysicalPosition {
                        x: position.x - previous_postion.x,
                        y: position.y - previous_postion.y,
                    };
                    if self.left_pressed {
                        self.yaw -= 0.02 * position_diff.x as f32;
                        self.pitch -= 0.02 * position_diff.y as f32;
                        needs_redraw = true;
                    }
                    if self.right_pressed {
                        self.center.x += 0.2
                            * (self.yaw.sin() * position_diff.x as f32
                                + self.yaw.cos() * position_diff.y as f32);
                        self.center.y -= 0.2
                            * (self.yaw.cos() * position_diff.x as f32
                                + self.yaw.sin() * position_diff.y as f32);
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
        create_model_view_projection(
            aspect_ratio,
            self.yaw,
            self.pitch,
            self.center,
            self.distance,
        )
    }
}

fn create_model_view_projection(
    aspect_ratio: f32,
    yaw: f32,
    pitch: f32,
    center: Vector3,
    distance: f32,
) -> CameraUniforms {
    let view_vector = -distance
        * Vector3::new(
            distance * yaw.cos() * pitch.sin(),
            distance * yaw.sin() * pitch.sin(),
            distance * pitch.cos(),
        );
    let projection_matrix = cgmath::perspective(cgmath::Deg(45f32), aspect_ratio, 1.0, 100.0);
    let position = Point3::new(
        center.x - view_vector.x,
        center.y - view_vector.y,
        center.z - view_vector.z,
    );
    let up = Vector3::unit_z();
    let view_matrix =
        cgmath::Matrix4::look_at(position, Point3::new(center.x, center.y, center.z), up);

    let model_view_projection_matrix = OPENGL_TO_WGPU_MATRIX * projection_matrix * view_matrix;

    CameraUniforms {
        view_matrix,
        model_view_projection_matrix,
        center,
        dummy0: 0.0,
        view_vector,
        dummy1: 0.0,
        position: position - Point3::new(0.0, 0.0, 0.0),
        dummy2: 0.0,
        up,
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
