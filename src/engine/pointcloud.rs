use std::{mem, path::PathBuf, sync::Arc};

use super::{camera::Camera, texture::Texture};

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use pcd_rs::{PcdDeserialize, Reader};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendComponent, BlendState, Buffer, BufferAddress,
    BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder,
    CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState, FrontFace, LoadOp,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PolygonMode, PrimitiveState, PrimitiveTopology, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState,
    StoreOp, SurfaceConfiguration, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};
use winit::window::Window;

#[allow(dead_code)]
#[derive(PcdDeserialize)]
struct Point {
    x: f32,
    y: f32,
    z: f32,
    intensity: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct Uniform {
    camera: [[f32; 4]; 4],
    resolution: [f32; 2],
    size: f32,
    _padding: u32,
}

impl Uniform {
    fn layout(device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("uniform_bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct Instance {
    model: [f32; 3],
}

impl Instance {
    fn layout() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as BufferAddress,
            step_mode: VertexStepMode::Instance,
            attributes: &[VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        }
    }
}

pub struct PointCloud {
    uniform_buffer: Buffer,
    uniform_bind_group: BindGroup,
    instances: Vec<Instance>,
    instance_buffer: Buffer,
    pipeline: RenderPipeline,
    point_size: f32,
}

impl PointCloud {
    pub fn new(
        device: &Device,
        camera: &Camera,
        window: Arc<Window>,
        config: &SurfaceConfiguration,
    ) -> Self {
        let point_size = 1.5;

        let uniform = Uniform {
            camera: camera.get_view_proj(),
            resolution: window.inner_size().into(),
            size: point_size,
            _padding: 0,
        };

        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pointcloud_uniform_buffer_layout"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("pointcloud_uniform_bind_group"),
            layout: &Uniform::layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let instances = Vec::new();

        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pointcloud_instance_buffer"),
            contents: &[],
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("pointcloud_shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/pointcloud.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pointcloud_pipeline_layout"),
            bind_group_layouts: &[&Uniform::layout(&device)],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("pointcloud_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                compilation_options: PipelineCompilationOptions::default(),
                entry_point: "vs_main",
                buffers: &[Instance::layout()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState {
                        color: BlendComponent::REPLACE,
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self {
            uniform_buffer,
            uniform_bind_group,
            instances,
            instance_buffer,
            pipeline,
            point_size,
        }
    }

    pub fn load_pcd(&mut self, path: &PathBuf, device: &Device) -> Result<()> {
        let points = match Self::read_pcd(path) {
            Ok(data) => data,
            Err(e) => return Err(e),
        };

        self.instances = Self::to_instance(&points);

        self.instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("pointcloud_instance_buffer"),
            contents: bytemuck::cast_slice(&self.instances),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        Ok(())
    }

    pub fn update(&self, camera: &Camera, queue: &Queue, window: &Window) {
        let uniform = Uniform {
            camera: camera.get_view_proj(),
            resolution: window.inner_size().into(),
            size: self.point_size,
            _padding: 0,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniform]));
    }

    pub fn draw(&self, encoder: &mut CommandEncoder, view: &TextureView, depth_texture: &Texture) {
        if self.instances.is_empty() {
            return;
        }

        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("pointcloud_render_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth_texture.view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        render_pass.draw(0..6, 0..self.instances.len() as _);
    }

    pub fn point_size(&self) -> f32 {
        self.point_size
    }

    pub fn set_point_size(&mut self, size: f32) {
        self.point_size = size;
    }

    fn read_pcd(path: &PathBuf) -> Result<Vec<Point>> {
        let reader = match Reader::open(path) {
            Ok(reader) => reader,
            Err(e) => return Err(e),
        };

        let points: Vec<Point> = match reader.collect() {
            Ok(points) => points,
            Err(e) => return Err(e),
        };

        Ok(points)
    }

    fn to_instance(points: &Vec<Point>) -> Vec<Instance> {
        let mut max_value = f32::MIN;

        for point in points {
            let tmp = point.x.max(point.y).max(point.z);
            if max_value < tmp {
                max_value = tmp;
            }
        }

        let mut instances = Vec::new();

        for point in points {
            let instance = Instance {
                model: [
                    point.x / max_value,
                    point.y / max_value,
                    point.z / max_value,
                ],
            };

            instances.push(instance);
        }

        instances
    }
}
