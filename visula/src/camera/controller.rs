use crate::camera::uniforms::CameraUniforms;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles};
use std::f32::consts::PI;
use web_time::Instant;
use winit::dpi::PhysicalSize;
use winit::window::{Window, WindowId};
use winit::{
    dpi::PhysicalPosition,
    event::{
        DeviceEvent, ElementState, MouseButton,
        MouseScrollDelta::{LineDelta, PixelDelta},
        TouchPhase, WindowEvent,
    },
};

#[derive(Copy, Clone, Debug, PartialEq)]
enum State {
    Released,
    PressedWaiting,
    Moving,
}

#[derive(Clone, Debug)]
pub struct CameraTransform {
    pub distance: f32,
    pub center: Vec3,
    pub forward: Vec3,
    pub true_up: Vec3,
    pub up: Vec3,
}

impl CameraTransform {
    pub fn position(&self) -> Vec3 {
        let view_vector = self.forward * self.distance;
        self.center - view_vector
    }
}

#[derive(Clone, Debug)]
pub enum DragPlane {
    Z,
    Camera,
}

#[derive(Debug)]
pub struct CameraController {
    left_pressed: bool,
    right_pressed: bool,
    control_pressed: bool,
    shift_pressed: bool,
    pub enabled: bool,
    pub zoom_enabled: bool,
    pub rotational_speed: f32,
    pub roll_speed: f32,
    state: State,
    window_id: WindowId,
    previous_time: Instant,
    pub current_transform: CameraTransform,
    pub target_transform: CameraTransform,
    pub smoothing: f32,
    last_screen_position: Option<PhysicalPosition<f64>>,
    window_size: PhysicalSize<u32>,
    first_intersection: Option<Vec3>,
    pub drag_plane: DragPlane,
}

#[derive(Clone, Debug)]
pub struct CameraControllerResponse {
    pub needs_redraw: bool,
    pub captured_event: bool,
}

fn lerp(current: Vec3, target: Vec3, rate: f32) -> Vec3 {
    current * (1.0 - rate) + rate * target
}

fn lerpf(current: f32, target: f32, rate: f32) -> f32 {
    current * (1.0 - rate) + rate * target
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
        let transform = CameraTransform {
            forward: new_forward,
            true_up: up,
            up,
            distance: 100.0,
            center: Vec3::new(0.0, 0.0, 0.0),
        };
        CameraController {
            enabled: true,
            zoom_enabled: true,
            left_pressed: false,
            right_pressed: false,
            control_pressed: false,
            shift_pressed: false,
            rotational_speed: 0.005 / scale_factor,
            roll_speed: 0.005 / scale_factor,
            state: State::Released,
            window_id,
            current_transform: transform.clone(),
            target_transform: transform.clone(),
            previous_time: Instant::now(),
            smoothing: 0.8,
            last_screen_position: None,
            window_size: window.inner_size(),
            first_intersection: None,
            drag_plane: DragPlane::Camera,
        }
    }

    pub fn update(&mut self) {
        let current_time = Instant::now();
        let dt = (current_time - self.previous_time).as_secs_f32();
        self.previous_time = current_time;

        let current_position = self.current_transform.position();
        let current_center = self.current_transform.center;
        let current_up = self.current_transform.up;
        let current_true_up = self.current_transform.true_up;
        let target_fps = 120.0;
        let smoothing_dt = 1.0 - self.smoothing.powf(target_fps * dt);
        let current_distance = self.current_transform.distance;
        let position = lerp(
            current_position,
            self.target_transform.position(),
            smoothing_dt,
        );
        let up = lerp(current_up, self.target_transform.up, smoothing_dt);
        let true_up = lerp(current_true_up, self.target_transform.true_up, smoothing_dt);
        let target_distance = self
            .target_transform
            .center
            .distance(self.target_transform.position());
        let distance = lerpf(current_distance, target_distance, smoothing_dt);
        let center = lerp(current_center, self.target_transform.center, smoothing_dt);
        let mut forward = center - position;
        if forward.length() == 0.0 {
            forward = Vec3::Z;
        }
        self.current_transform.forward = forward.normalize();
        self.current_transform.up = up;
        self.current_transform.true_up = true_up;
        self.current_transform.distance = distance;
        self.current_transform.center = center;
    }

    pub fn device_event(&mut self, event: &DeviceEvent) -> CameraControllerResponse {
        let mut response = CameraControllerResponse {
            needs_redraw: false,
            captured_event: false,
        };
        if !self.enabled {
            return response;
        }
        if self.shift_pressed {
            return response;
        }
        let up = self.target_transform.up.normalize();
        let forward = self.target_transform.forward.normalize();
        let right = Vec3::cross(forward, up).normalize();
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
                    self.target_transform.up = (rotation * self.target_transform.up).normalize();
                    self.target_transform.true_up =
                        (rotation * self.target_transform.true_up).normalize();
                    self.target_transform.forward =
                        (rotation * self.target_transform.forward).normalize();
                } else {
                    if (position_diff.x + position_diff.y).abs() < 0.000001 {
                        return CameraControllerResponse {
                            needs_redraw: false,
                            captured_event: false,
                        };
                    }
                    let rotation_x = Quat::from_axis_angle(
                        self.target_transform.true_up,
                        -self.rotational_speed * position_diff.x,
                    );
                    let rotation_y =
                        Quat::from_axis_angle(right, -self.rotational_speed * position_diff.y);
                    self.target_transform.forward =
                        (rotation_x * rotation_y * self.target_transform.forward).normalize();
                    self.target_transform.up =
                        (rotation_x * rotation_y * self.target_transform.up).normalize();
                }
                response.needs_redraw = true;
                response.captured_event = true;
                self.state = State::Moving;
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
            WindowEvent::Resized(size) => {
                self.window_size = *size;
            }
            WindowEvent::ModifiersChanged(state) => {
                self.control_pressed = state
                    .state()
                    .contains(winit::keyboard::ModifiersState::CONTROL);
            }
            WindowEvent::MouseWheel { delta, .. } if self.zoom_enabled => {
                let diff = match delta {
                    LineDelta(_x, y) => *y,
                    PixelDelta(delta) => 0.04 * delta.y as f32,
                };
                let factor = 1.0 + 0.1 * diff.abs();
                if diff > 0.0 {
                    self.target_transform.distance /= factor;
                } else {
                    self.target_transform.distance *= factor;
                }
                response.needs_redraw = true;
                response.captured_event = true;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.right_pressed || (self.left_pressed && self.shift_pressed) {
                    let ndc_ray = Vec4::new(
                        2.0 * position.x as f32 / self.window_size.width as f32 - 1.0,
                        1.0 - 2.0 * position.y as f32 / self.window_size.height as f32,
                        -1.0,
                        1.0,
                    );
                    let mut camera_ray = self
                        .projection_matrix(
                            self.window_size.width as f32 / self.window_size.height as f32,
                        )
                        .inverse()
                        * ndc_ray;
                    camera_ray.w = 0.0;
                    let world_ray = (self.view_matrix().inverse() * camera_ray)
                        .xyz()
                        .normalize();
                    let camera_position = self.target_transform.position();
                    let t = match self.drag_plane {
                        DragPlane::Z => -camera_position.y / world_ray.y,
                        DragPlane::Camera => {
                            self.target_transform.distance
                                / self.target_transform.forward.dot(world_ray)
                        }
                    };
                    let intersection = camera_position + t * world_ray;
                    match self.first_intersection {
                        None => {
                            self.first_intersection = Some(intersection);
                        }
                        Some(first_intersection) => {
                            self.target_transform.center =
                                first_intersection - intersection + self.target_transform.center;
                        }
                    }
                }
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
                        self.first_intersection = None;
                    }
                },
                _ => {}
            },
            WindowEvent::Touch(touch) => match touch.phase {
                TouchPhase::Started => {
                    self.left_pressed = true;
                    self.state = State::PressedWaiting;
                }
                TouchPhase::Ended => {
                    self.left_pressed = false;
                    self.state = State::Released;
                }
                TouchPhase::Moved => {
                    let up = self.target_transform.up.normalize();
                    let forward = self.target_transform.forward.normalize();
                    let right = Vec3::cross(forward, up).normalize();

                    let position_diff = match self.last_screen_position {
                        None => Vec2::ZERO,
                        Some(last) => Vec2 {
                            x: (touch.location.x - last.x) as f32,
                            y: (touch.location.y - last.y) as f32,
                        },
                    };
                    if (position_diff.x + position_diff.y).abs() < 0.000001 {
                        return CameraControllerResponse {
                            needs_redraw: false,
                            captured_event: false,
                        };
                    }
                    let rotation_x = Quat::from_axis_angle(
                        self.target_transform.true_up,
                        -self.rotational_speed * position_diff.x,
                    );
                    let rotation_y =
                        Quat::from_axis_angle(right, -self.rotational_speed * position_diff.y);
                    self.target_transform.forward =
                        (rotation_x * rotation_y * self.target_transform.forward).normalize();
                    self.target_transform.up =
                        (rotation_x * rotation_y * self.target_transform.up).normalize();
                    self.last_screen_position = Some(touch.location);
                }
                TouchPhase::Cancelled => {
                    self.left_pressed = false;
                    self.state = State::Released;
                }
            },
            _ => {}
        }
        response
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(
            self.position(),
            self.current_transform.center,
            self.current_transform.up,
        )
    }

    pub fn position(&self) -> Vec3 {
        self.current_transform.position()
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
            center: self.current_transform.center,
            dummy0: 0.0,
            view_vector: self.current_transform.forward * self.current_transform.distance,
            dummy1: 0.0,
            position: self.current_transform.position() - Vec3::ZERO,
            dummy2: 0.0,
            up: self.current_transform.up,
            dummy3: 0.0,
        }
    }
}
