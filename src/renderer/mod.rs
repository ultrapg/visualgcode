use std::sync::{Arc, RwLock};

use bytemuck::{Pod, Zeroable};
use egui_wgpu::{CallbackResources, CallbackTrait};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, PrimitiveTopology, ShaderStages,
};

use crate::parser;

pub mod camera;

pub struct RenderCallback {
    pub shared: Arc<RwLock<SharedState>>,
}

impl RenderCallback {
    pub fn new(shared: Arc<RwLock<SharedState>>) -> Self {
        Self { shared }
    }
}

pub struct SharedState {
    pub program: Option<parser::GCodeProgram>,
    pub view_proj: [[f32; 4]; 4],
    pub layer_min: u32,
    pub layer_max: u32,
    pub max_segment_index: u32,
    pub needs_upload: bool,
    pub target_format: Option<wgpu::TextureFormat>,
}

impl SharedState {
    pub fn new(program: Option<parser::GCodeProgram>) -> Self {
        Self {
            program,
            view_proj: camera::mat4_identity(),
            layer_min: 0,
            layer_max: u32::MAX,
            max_segment_index: u32::MAX,
            needs_upload: false,
            target_format: None,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GcodeVertex {
    pub position: [f32; 3],
    pub is_drawing: u32,
    pub layer_index: u32,
    pub segment_index: u32,
}

impl GcodeVertex {
    const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<u32>())
                        as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Uint32,
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() + 2 * std::mem::size_of::<u32>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GcodeUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub layer_min: u32,
    pub layer_max: u32,
    pub max_segment_index: u32,
    pub _padding: u32,
}

pub struct RenderResources {
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub uniform_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub has_data: bool,
}

const SHADER: &str = "
struct Uniforms {
    view_proj: mat4x4<f32>,
    layer_min: u32,
    layer_max: u32,
    max_segment_index: u32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) is_drawing: u32,
    @location(2) layer_index: u32,
    @location(3) segment_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    if (input.layer_index < uniforms.layer_min ||
        input.layer_index > uniforms.layer_max ||
        input.segment_index > uniforms.max_segment_index) {
        output.position = vec4<f32>(0.0, 0.0, 2000000.0, 1.0);
    } else {
        output.position = uniforms.view_proj * vec4<f32>(input.position, 1.0);
    }
    if (input.is_drawing == 1u) {
        output.color = vec3<f32>(0.15, 0.55, 1.0);
    } else {
        output.color = vec3<f32>(0.45, 0.45, 0.45);
    }
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
";

pub fn build_vertices(program: &parser::GCodeProgram) -> Vec<GcodeVertex> {
    let mut vertices = Vec::with_capacity(program.segments.len() * 2);
    for seg in &program.segments {
        vertices.push(GcodeVertex {
            position: [seg.start.x, seg.start.y, seg.start.z],
            is_drawing: seg.is_drawing as u32,
            layer_index: seg.layer_index,
            segment_index: seg.segment_index,
        });
        vertices.push(GcodeVertex {
            position: [seg.end.x, seg.end.y, seg.end.z],
            is_drawing: seg.is_drawing as u32,
            layer_index: seg.layer_index,
            segment_index: seg.segment_index,
        });
    }
    vertices
}

pub fn create_render_resources(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
) -> RenderResources {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("gcode_shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER)),
    });

    let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("uniform_buffer"),
        contents: bytemuck::cast_slice(&[GcodeUniforms {
            view_proj: camera::mat4_identity(),
            layer_min: 0,
            layer_max: u32::MAX,
            max_segment_index: u32::MAX,
            _padding: 0,
        }]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("uniform_bind_group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("gcode_pipeline_layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gcode_pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[GcodeVertex::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Cw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let empty_vertex_data: [GcodeVertex; 0] = [];
    let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("vertex_buffer"),
        contents: bytemuck::cast_slice(&empty_vertex_data),
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
    });

    RenderResources {
        pipeline,
        vertex_buffer,
        num_vertices: 0,
        uniform_buffer,
        bind_group,
        has_data: false,
    }
}



impl CallbackTrait for RenderCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        // Ensure resources exist (they may have been lost if renderer was recreated)
        if callback_resources.get::<RenderResources>().is_none() {
            let target_format = self.shared.read().unwrap().target_format;
            if let Some(fmt) = target_format {
                let new_resources = create_render_resources(device, fmt);
                callback_resources.insert(new_resources);
            }
        }

        let resources = match callback_resources.get_mut::<RenderResources>() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Snapshot the shared state
        let (program, needs_upload, uniforms) = {
            let state = self.shared.read().unwrap();
            (
                state.program.clone(),
                state.needs_upload || !resources.has_data,
                GcodeUniforms {
                    view_proj: state.view_proj,
                    layer_min: state.layer_min,
                    layer_max: state.layer_max,
                    max_segment_index: state.max_segment_index,
                    _padding: 0,
                },
            )
        };

        if let Some(ref program) = program {
            if needs_upload {
                let vertices = build_vertices(program);
                let vertex_size =
                    std::mem::size_of_val(&*vertices) as wgpu::BufferAddress;

                if vertex_size > 0 {
                    if resources.vertex_buffer.size() < vertex_size {
                        let new_buffer = device.create_buffer_init(&BufferInitDescriptor {
                            label: Some("vertex_buffer"),
                            contents: bytemuck::cast_slice(&vertices),
                            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                        });
                        resources.vertex_buffer = new_buffer;
                    } else {
                        queue.write_buffer(
                            &resources.vertex_buffer,
                            0,
                            bytemuck::cast_slice(&vertices),
                        );
                    }
                    resources.num_vertices = vertices.len() as u32;
                }
                resources.has_data = true;
                let mut state = self.shared.write().unwrap();
                state.needs_upload = false;
            }

            queue.write_buffer(
                &resources.uniform_buffer,
                0,
                bytemuck::cast_slice(&[uniforms]),
            );
        }

        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        let resources = match callback_resources.get::<RenderResources>() {
            Some(r) => r,
            None => return,
        };
        if !resources.has_data || resources.num_vertices == 0 {
            return;
        }
        render_pass.set_pipeline(&resources.pipeline);
        render_pass.set_bind_group(0, &resources.bind_group, &[]);
        render_pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
        render_pass.draw(0..resources.num_vertices, 0..1);
    }
}
