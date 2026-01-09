use std::{collections::HashSet, sync::Arc};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

mod rendering;
use rendering::Renderer;

pub struct App {
    renderer: Option<Renderer>,
    last_frame: std::time::Instant,
    pressed_keys: HashSet<KeyCode>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            renderer: None,
            last_frame: std::time::Instant::now(),
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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let renderer = match &mut self.renderer {
            Some(s) => s,
            None => return,
        };

        const MOVE_DIST: f32 = 5.;
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => match renderer.render() {
                Ok(_) => {
                    let after = std::time::Instant::now();
                    println!("{} fps", 1_000_000 / (after - self.last_frame).as_micros());
                    self.last_frame = after;

                    for code in &self.pressed_keys {
                        match code {
                            KeyCode::KeyA => renderer.camera_left_right(-MOVE_DIST),
                            KeyCode::KeyD => renderer.camera_left_right(MOVE_DIST),
                            KeyCode::KeyW => renderer.camera_up_down(MOVE_DIST),
                            KeyCode::KeyS => renderer.camera_up_down(-MOVE_DIST),
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
            } => match (code, key_state.is_pressed()) {
                (KeyCode::KeyQ, true) => event_loop.exit(),
                (code, true) => {
                    self.pressed_keys.insert(code);
                    renderer.window.request_redraw();
                }
                (code, false) => {
                    self.pressed_keys.remove(&code);
                    renderer.window.request_redraw();
                }
            },
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
