mod engine;

use engine::Engine;
use wgpu::SurfaceError;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("pcvisualizer")
        .with_inner_size(LogicalSize::new(1280, 720))
        .build(&event_loop)
        .unwrap();

    let mut engine = Engine::new(&window).await;
    let mut surface_configured = false;

    event_loop
        .run(move |event, control_flow| match event {
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == engine.window().id() => {
                if !engine.input(event) {
                    match event {
                        WindowEvent::CloseRequested => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => {
                            surface_configured = true;
                            engine.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            engine.window().request_redraw();

                            if !surface_configured {
                                return;
                            }

                            engine.update();
                            match engine.render() {
                                Ok(_) => {}
                                Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                                    engine.resize(engine.size())
                                }
                                Err(SurfaceError::OutOfMemory | SurfaceError::Timeout) => {
                                    control_flow.exit()
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        })
        .unwrap();
}
