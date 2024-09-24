mod camera;
mod gui;
mod pointcloud;
mod texture;

use camera::Camera;
use egui_wgpu::ScreenDescriptor;
use gui::{layout, EguiRender};
use pointcloud::PointCloud;
use texture::Texture;
use wgpu::{
    Backends, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, Operations, PowerPreference, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError,
    TextureUsages, TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub struct Engine<'a> {
    size: PhysicalSize<u32>,
    surface: Surface<'a>,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    depth_texture: Texture,
    gui: EguiRender,
    window: &'a Window,
    camera: Camera,
    pointcloud: PointCloud,
}

impl<'a> Engine<'a> {
    pub async fn new(window: &'a Window) -> Self {
        let size = window.inner_size();

        let instance = Instance::new(InstanceDescriptor::default());

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let camera = Camera::new(
            (0.5, 0.5, 0.5).into(),
            (0.0, 0.0, 0.0).into(),
            (0.0, 0.0, 1.0).into(),
            config.width as f32 / config.height as f32,
            45.0,
        );

        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        let gui = EguiRender::new(&device, config.format, None, 1, &window);

        let pointcloud = PointCloud::new(&device, &camera, window, &config);

        Self {
            size,
            surface,
            config,
            device,
            queue,
            depth_texture,
            gui,
            window,
            camera,
            pointcloud,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        if self.gui.input(&self.window, &event) {
            return true;
        }

        if self.camera.process_event(event, &self.window) {
            return true;
        }

        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match keycode {
                KeyCode::KeyJ => self
                    .pointcloud
                    .set_point_size(self.pointcloud.point_size() - 0.1),
                KeyCode::KeyK => self
                    .pointcloud
                    .set_point_size(self.pointcloud.point_size() + 0.1),
                _ => return false,
            },
            WindowEvent::DroppedFile(path) => {
                match self.pointcloud.load_pcd(path, &self.device) {
                    Err(e) => eprintln!("{:?}", e),
                    _ => {}
                }
                self.window.request_redraw();
            }
            _ => return false,
        }

        true
    }

    pub fn update(&mut self) {
        self.pointcloud
            .update(&self.camera, &self.queue, &self.window);
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera
                .set_aspect(new_size.width as f32 / new_size.height as f32);
            self.depth_texture =
                Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub fn render(&mut self) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let _ = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("init_render_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: wgpu::LoadOp::Clear(Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.pointcloud
            .draw(&mut encoder, &view, &self.depth_texture);

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        self.gui.draw(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.window,
            &view,
            screen_descriptor,
            |ui| layout(ui),
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
