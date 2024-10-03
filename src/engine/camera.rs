use cgmath::{Deg, InnerSpace, Matrix4, Point3, Vector3};
use winit::{
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

pub struct Camera {
    eye: Point3<f32>,
    target: Point3<f32>,
    up: Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,

    mouse_right_position: Option<(f32, f32)>,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
}

impl Camera {
    pub fn new(
        eye: Point3<f32>,
        target: Point3<f32>,
        up: Vector3<f32>,
        aspect: f32,
        fovy: f32,
    ) -> Self {
        Self {
            eye,
            target,
            up,
            aspect,
            fovy,
            znear: 0.01,
            zfar: 100.0,
            mouse_right_position: None,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
        }
    }

    pub fn process_event(&mut self, event: &WindowEvent, window: &Window) -> bool {
        match event {
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => match state {
                ElementState::Pressed => self.mouse_right_position = Some((0.0, 0.0)),
                ElementState::Released => self.mouse_right_position = None,
            },
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_x, y) => {
                    self.camera_zoom(*y);
                }
                _ => return false,
            },
            WindowEvent::CursorMoved { position, .. } => {
                if let Some((x, y)) = self.mouse_right_position {
                    let logical_position = position.to_logical(window.scale_factor());
                    if x == 0.0 && y == 0.0 {
                        self.mouse_right_position = Some((logical_position.x, logical_position.y));
                    } else {
                        self.camera_rotate(logical_position.x - x, logical_position.y - y);
                    }
                } else {
                    return false;
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => self.is_up_pressed = is_pressed,
                    KeyCode::KeyS | KeyCode::ArrowDown => self.is_down_pressed = is_pressed,
                    KeyCode::KeyA | KeyCode::ArrowLeft => self.is_left_pressed = is_pressed,
                    KeyCode::KeyD | KeyCode::ArrowRight => self.is_right_pressed = is_pressed,
                    KeyCode::KeyB => self.set_birdeye(),
                    _ => return false,
                }
            }
            _ => return false,
        }

        true
    }

    pub fn get_view_proj(&self) -> [[f32; 4]; 4] {
        self.build_view_projection_matrix().into()
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    fn camera_rotate(&mut self, delta_x: f32, delta_y: f32) {
        let forward = self.target - self.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        let right = forward_norm.cross(self.up).normalize();
        let up = self.up.normalize();

        let scale = 0.0001;

        let eye = self.target
            - (forward + right * delta_x * scale + up * delta_y * scale).normalize() * forward_mag;

        if eye.z > 0.0 {
            self.eye = eye;
        }
    }

    fn camera_zoom(&mut self, y: f32) {
        let forward = self.target - self.eye;
        let forward_norm = forward.normalize();

        let scale = 0.01;

        let eye = self.eye + forward_norm * y * scale;

        if eye.z > 0.0 {
            self.eye = eye;
        }
    }

    fn set_birdeye(&mut self) {
        self.camera_rotate(0.0, -1e8);
    }
}
