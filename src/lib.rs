use graphics::{
    draw_state::Blend, types::Color, Context, DrawState, Graphics, ImageSize, Viewport,
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexInput {
    position: [f32; 2],
    color: [f32; 4],
}

impl VertexInput {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<VertexInput>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

struct PsoBlend<T> {
    none: T,
    alpha: T,
    add: T,
    lighter: T,
    multiply: T,
    invert: T,
}

impl<T> PsoBlend<T> {
    fn new<F>(mut f: F) -> Self
    where
        F: FnMut(Option<wgpu::BlendState>) -> T,
    {
        let none = f(None);
        let alpha = f(Some(wgpu::BlendState::ALPHA_BLENDING));
        let add = f(Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        }));
        let lighter = f(Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        }));
        let multiply = f(Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Dst,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::DstAlpha,
                dst_factor: wgpu::BlendFactor::Zero,
                operation: wgpu::BlendOperation::Add,
            },
        }));
        let invert = f(Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Constant,
                dst_factor: wgpu::BlendFactor::Src,
                operation: wgpu::BlendOperation::Subtract,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        }));

        Self {
            alpha,
            add,
            multiply,
            invert,
            none,
            lighter,
        }
    }

    fn blend(&self, blend: Option<Blend>) -> &T {
        match blend {
            None => &self.none,
            Some(Blend::Alpha) => &self.alpha,
            Some(Blend::Add) => &self.add,
            Some(Blend::Lighter) => &self.lighter,
            Some(Blend::Multiply) => &self.multiply,
            Some(Blend::Invert) => &self.invert,
        }
    }
}

pub struct Texture {}

impl ImageSize for Texture {
    fn get_size(&self) -> (u32, u32) {
        todo!()
    }
}

pub struct Wgpu2d {
    render_pipelines: PsoBlend<wgpu::RenderPipeline>,
}

impl Wgpu2d {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let shader_module = device.create_shader_module(&wgpu::include_wgsl!("shader.wgsl"));

        let render_pipelines = PsoBlend::new(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "main",
                    buffers: &[VertexInput::desc()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    clamp_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "main",
                    targets: &[wgpu::ColorTargetState {
                        format: config.format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
            })
        });

        Self { render_pipelines }
    }

    pub fn draw<F>(
        &mut self,
        device: &wgpu::Device,
        output_view: &wgpu::TextureView,
        viewport: Viewport,
        f: F,
    ) -> wgpu::CommandBuffer
    where
        F: FnOnce(Context, &mut WgpuGraphics),
    {
        let mut g = WgpuGraphics::new(self);
        let c = Context::new_viewport(viewport);
        f(c, &mut g);
        g.draw(device, output_view)
    }
}

pub struct WgpuGraphics<'a> {
    wgpu2d: &'a Wgpu2d,
    clear_color: Option<Color>,
    vertices: Vec<(DrawState, Vec<VertexInput>)>,
}

impl<'a> WgpuGraphics<'a> {
    pub fn new(wgpu2d: &'a Wgpu2d) -> Self {
        Self {
            wgpu2d,
            clear_color: None,
            vertices: vec![],
        }
    }

    pub fn draw(
        self,
        device: &wgpu::Device,
        output_view: &wgpu::TextureView,
    ) -> wgpu::CommandBuffer {
        let load = match self.clear_color {
            Some(c) => wgpu::LoadOp::Clear(to_wgpu_color(c)),
            None => wgpu::LoadOp::Load,
        };

        let draw_states_vertex_buffers_and_counts = self
            .vertices
            .iter()
            .map(|(draw_state, vertices)| {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                (draw_state, vertex_buffer, vertices.len())
            })
            .collect::<Vec<_>>();

        encode(device, |encoder| {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load, store: true },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_blend_constant(wgpu::Color::WHITE);

            for (draw_state, vertex_buffer, count) in &draw_states_vertex_buffers_and_counts {
                render_pass.set_pipeline(&self.wgpu2d.render_pipelines.blend(draw_state.blend));
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.draw(0..*count as u32, 0..1);
            }
        })
    }
}

impl<'a> Graphics for WgpuGraphics<'a> {
    type Texture = Texture;

    fn clear_color(&mut self, color: Color) {
        self.clear_color = Some(color);
        self.vertices.clear();
    }

    fn clear_stencil(&mut self, value: u8) {}

    fn tri_list<F>(&mut self, draw_state: &DrawState, &color: &[f32; 4], mut f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]])),
    {
        if self.vertices.last().map(|&(s, _)| s) != Some(*draw_state) {
            self.vertices.push((*draw_state, vec![]))
        };
        f(&mut |positions| {
            for &position in positions {
                self.vertices
                    .last_mut()
                    .unwrap()
                    .1
                    .push(VertexInput { position, color });
            }
        });
    }

    fn tri_list_c<F>(&mut self, draw_state: &DrawState, mut f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]], &[[f32; 4]])),
    {
        if self.vertices.last().map(|&(s, _)| s) != Some(*draw_state) {
            self.vertices.push((*draw_state, vec![]))
        };
        f(&mut |positions, colors| {
            for (&position, &color) in positions.iter().zip(colors.iter()) {
                self.vertices
                    .last_mut()
                    .unwrap()
                    .1
                    .push(VertexInput { position, color });
            }
        });
    }

    fn tri_list_uv<F>(&mut self, draw_state: &DrawState, color: &[f32; 4], texture: &Texture, f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]], &[[f32; 2]])),
    {
        todo!()
    }

    fn tri_list_uv_c<F>(&mut self, draw_state: &DrawState, texture: &Texture, f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]], &[[f32; 2]], &[[f32; 4]])),
    {
        todo!()
    }
}

fn encode<F>(device: &wgpu::Device, f: F) -> wgpu::CommandBuffer
where
    F: FnOnce(&mut wgpu::CommandEncoder),
{
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Command Encoder"),
    });

    f(&mut encoder);

    encoder.finish()
}

fn to_wgpu_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: color[0] as f64,
        g: color[1] as f64,
        b: color[2] as f64,
        a: color[3] as f64,
    }
}
