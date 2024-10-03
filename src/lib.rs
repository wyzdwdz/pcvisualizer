mod engine;

use std::path::PathBuf;

use engine::Engine;
use wgpu::SurfaceError;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default)]
struct App {
    engine: Option<Engine>,
    pcd_path: Option<PathBuf>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attrs = Window::default_attributes()
            .with_title("pcvisualizer")
            .with_inner_size(LogicalSize::new(1280, 720));

        let window = event_loop.create_window(window_attrs).unwrap();
        self.engine = Some(Engine::new(window));

        let Some(ref mut engine) = self.engine else {
            return;
        };

        match &self.pcd_path {
            Some(path) => engine.set_pcd(&path),
            None => (),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(ref mut engine) = self.engine else {
            return;
        };

        if window_id != engine.window().id() {
            return;
        }

        if engine.input(&event) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(physical_size) => {
                engine.resize(physical_size);
            }

            WindowEvent::RedrawRequested => {
                engine.update();
                match engine.render() {
                    Ok(_) => {}
                    Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                        engine.resize(engine.size())
                    }
                    Err(SurfaceError::OutOfMemory | SurfaceError::Timeout) => event_loop.exit(),
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(ref mut engine) = self.engine else {
            return;
        };

        engine.window().request_redraw();
    }
}

pub fn run(pcd_path: Option<PathBuf>) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    app.pcd_path = pcd_path;
    let _ = event_loop.run_app(&mut app);
}
