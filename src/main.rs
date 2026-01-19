use std::{collections::HashSet, sync::Arc};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{DeviceEvent, DeviceId, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod rendering;
use rendering::Renderer;

pub struct App {
    renderer: Option<Renderer>,
    last_time: std::time::Instant,
    pressed_keys: HashSet<KeyCode>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            renderer: None,
            last_time: std::time::Instant::now(),
            pressed_keys: HashSet::new(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("Voxel Engine");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.renderer = Some(pollster::block_on(Renderer::new(window)).unwrap());
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        let renderer = match &mut self.renderer {
            Some(s) => s,
            None => return,
        };
        match event {
            DeviceEvent::MouseMotion { delta: (x, y) } => {
                let rot_mult = 0.2;
                renderer.rot_y(x as f32 * rot_mult);
                renderer.rot_x(y as f32 * rot_mult);
            }
            _ => {}
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let renderer = match &mut self.renderer {
            Some(s) => s,
            None => return,
        };
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::CursorEntered { .. } => {
                renderer.window.set_cursor_visible(false);
                if let Err(err) = renderer
                    .window
                    .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                {
                    panic!("Error setting cursor grab: {err}");
                }
            }
            WindowEvent::RedrawRequested => match renderer.render() {
                Ok(_) => {
                    let after = std::time::Instant::now();
                    let delta_time = after - self.last_time;

                    self.last_time = after;

                    let mult = if self.pressed_keys.contains(&KeyCode::ShiftLeft) {
                        3.
                    } else if self.pressed_keys.contains(&KeyCode::ControlLeft) {
                        0.25
                    } else {
                        1.
                    };
                    let move_dist = 20. * mult * delta_time.as_secs_f32();
                    let rot_dist = 80. * mult * delta_time.as_secs_f32();
                    for code in &self.pressed_keys {
                        match code {
                            KeyCode::KeyW => renderer.camera_forward_back(move_dist),
                            KeyCode::KeyS => renderer.camera_forward_back(-move_dist),
                            KeyCode::KeyA => renderer.camera_left_right(-move_dist),
                            KeyCode::KeyD => renderer.camera_left_right(move_dist),
                            KeyCode::ArrowLeft => renderer.rot_z(rot_dist),
                            KeyCode::ArrowRight => renderer.rot_z(-rot_dist),
                            KeyCode::KeyI => {
                                renderer.camera.fov = (renderer.camera.fov + 1.)
                                    .clamp(0_f32.next_up(), 180_f32.next_down());
                                renderer.window.request_redraw();
                            }
                            KeyCode::KeyK => {
                                renderer.camera.fov = (renderer.camera.fov - 1.)
                                    .clamp(0_f32.next_up(), 180_f32.next_down());
                                renderer.window.request_redraw();
                            }
                            KeyCode::KeyR => renderer.reset_camera(),
                            _ => {}
                        }
                    }
                }
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    let size = renderer.window.inner_size();
                    renderer.resize(size.width, size.height);
                }
                Err(e) => {
                    println!("Unable to render {}", e);
                }
            },
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                self.last_time = std::time::Instant::now();
                match (code, key_state.is_pressed()) {
                    (KeyCode::KeyQ, true) => event_loop.exit(),
                    (KeyCode::Escape, true) => {
                        renderer
                            .window
                            .set_cursor_grab(winit::window::CursorGrabMode::None)
                            .unwrap();
                        renderer.window.set_cursor_visible(true);
                    }
                    (code, true) => {
                        self.pressed_keys.insert(code);
                        renderer.window.request_redraw();
                    }
                    (code, false) => {
                        self.pressed_keys.remove(&code);
                        renderer.window.request_redraw();
                    }
                }
            }
            WindowEvent::Focused(false) => self.pressed_keys.clear(),
            _ => {}
        }
    }
}

fn main() -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();
    event_loop.run_app(&mut app)
}
