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

pub struct Wgpu2d<'a> {
    device: &'a wgpu::Device,
    render_pipelines: PsoBlend<wgpu::RenderPipeline>,
}

impl<'a> Wgpu2d<'a> {
    pub fn new<'b>(device: &'a wgpu::Device, config: &'b wgpu::SurfaceConfiguration) -> Self {
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

        Self {
            device,
            render_pipelines,
        }
    }

    pub fn draw<F>(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        output_view: &wgpu::TextureView,
        viewport: Viewport,
        f: F,
    ) -> wgpu::CommandBuffer
    where
        F: FnOnce(Context, &mut WgpuGraphics),
    {
        let mut g = WgpuGraphics::new(self, config);
        let c = Context::new_viewport(viewport);
        f(c, &mut g);
        g.draw(device, output_view)
    }
}

pub struct WgpuGraphics<'a> {
    wgpu2d: &'a Wgpu2d<'a>,
    color_format: wgpu::TextureFormat,
    clear_color: Option<Color>,
    render_bundles: Vec<wgpu::RenderBundle>,
}

impl<'a> WgpuGraphics<'a> {
    pub fn new(wgpu2d: &'a Wgpu2d<'a>, config: &wgpu::SurfaceConfiguration) -> Self {
        Self {
            wgpu2d,
            color_format: config.format,
            clear_color: None,
            render_bundles: vec![],
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

            render_pass.execute_bundles(self.render_bundles.iter());
        })
    }

    fn bundle<'b, F>(&self, device: &'b wgpu::Device, f: F) -> wgpu::RenderBundle
    where
        F: FnOnce(&mut dyn wgpu::util::RenderEncoder<'b>),
    {
        let mut render_bundle_encoder =
            device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                label: Some("Render Bundle Encoder"),
                color_formats: &[self.color_format],
                depth_stencil: None,
                sample_count: 1,
            });

        f(&mut render_bundle_encoder);

        render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Render Bundle"),
        })
    }

    fn bundle_colored(&mut self, vertex_inputs: &[VertexInput], draw_state: &DrawState) {
        let vertex_buffer =
            self.wgpu2d
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertex_inputs),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let render_bundle = self.bundle(self.wgpu2d.device, |render_encoder| {
            render_encoder.set_pipeline(&self.wgpu2d.render_pipelines.blend(draw_state.blend));
            render_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_encoder.draw(0..vertex_inputs.len() as u32, 0..1);
        });

        self.render_bundles.push(render_bundle);
    }
}

impl<'a> Graphics for WgpuGraphics<'a> {
    type Texture = Texture;

    fn clear_color(&mut self, color: Color) {
        self.clear_color = Some(color);
        self.render_bundles.clear();
    }

    fn clear_stencil(&mut self, value: u8) {}

    fn tri_list<F>(&mut self, draw_state: &DrawState, &color: &[f32; 4], mut f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]])),
    {
        f(&mut |positions| {
            let pipeline_inputs = positions
                .iter()
                .map(|&position| VertexInput { position, color })
                .collect::<Vec<_>>();

            self.bundle_colored(&pipeline_inputs, draw_state);
        });
    }

    fn tri_list_c<F>(&mut self, draw_state: &DrawState, mut f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]], &[[f32; 4]])),
    {
        f(&mut |positions, colors| {
            let pipeline_inputs = positions
                .iter()
                .zip(colors.iter())
                .map(|(&position, &color)| VertexInput { position, color })
                .collect::<Vec<_>>();

            self.bundle_colored(&pipeline_inputs, draw_state);
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
