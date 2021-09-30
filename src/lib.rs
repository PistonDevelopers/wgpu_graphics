use graphics::{
    draw_state::Blend, types::Color, Context, DrawState, Graphics, ImageSize, Viewport,
};
use std::{
    fmt::{self, Display, Formatter},
    path::Path,
};
use texture::{CreateTexture, Format, TextureOp, TextureSettings};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ColoredPipelineInput {
    position: [f32; 2],
    color: [f32; 4],
}

impl ColoredPipelineInput {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ColoredPipelineInput>() as wgpu::BufferAddress,
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

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TexturedPipelineInput {
    xy: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

impl TexturedPipelineInput {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TexturedPipelineInput>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
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
        use wgpu::{BlendComponent, BlendFactor::*, BlendOperation::*, BlendState};

        let none = f(None);
        let alpha = f(Some(BlendState::ALPHA_BLENDING));
        let add = f(Some(BlendState {
            color: BlendComponent {
                src_factor: One,
                dst_factor: One,
                operation: Add,
            },
            alpha: BlendComponent {
                src_factor: One,
                dst_factor: One,
                operation: Add,
            },
        }));
        let lighter = f(Some(BlendState {
            color: BlendComponent {
                src_factor: SrcAlpha,
                dst_factor: One,
                operation: Add,
            },
            alpha: BlendComponent {
                src_factor: Zero,
                dst_factor: One,
                operation: Add,
            },
        }));
        let multiply = f(Some(BlendState {
            color: BlendComponent {
                src_factor: Dst,
                dst_factor: Zero,
                operation: Add,
            },
            alpha: BlendComponent {
                src_factor: DstAlpha,
                dst_factor: Zero,
                operation: Add,
            },
        }));
        let invert = f(Some(BlendState {
            color: BlendComponent {
                src_factor: Constant,
                dst_factor: Src,
                operation: Subtract,
            },
            alpha: BlendComponent {
                src_factor: Zero,
                dst_factor: One,
                operation: Add,
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

struct PsoStencil<T> {
    none: T,
    clip: T,
    inside: T,
    outside: T,
    increment: T,
}

impl<T> PsoStencil<T> {
    fn new<F>(f: F) -> PsoStencil<PsoBlend<T>>
    where
        F: FnMut(Option<wgpu::BlendState>, Option<wgpu::StencilState>),
    {
        todo!()
    }
}

pub struct Texture {
    texture: wgpu::Texture,
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

pub struct TextureContext<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl<'a> TextureContext<'a> {
    pub fn from_parts(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        TextureContext { device, queue }
    }
}

impl Texture {
    pub fn from_path<'a, P>(
        context: &mut TextureContext<'a>,
        path: P,
        settings: &TextureSettings,
    ) -> Result<Self, TextureError>
    where
        P: AsRef<Path>,
    {
        let img = image::open(path).map_err(TextureError::ImageError)?;
        let img = match img {
            image::DynamicImage::ImageRgba8(img) => img,
            img => img.to_rgba8(),
        };

        Texture::from_image(context, &img, settings)
    }

    pub fn from_image<'a>(
        context: &mut TextureContext<'a>,
        img: &image::RgbaImage,
        settings: &TextureSettings,
    ) -> Result<Self, TextureError> {
        let (width, height) = img.dimensions();
        CreateTexture::create(context, Format::Rgba8, img, [width, height], settings)
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        })
    }
}

impl<'a> TextureOp<TextureContext<'a>> for Texture {
    type Error = TextureError;
}

#[derive(Debug)]
pub enum TextureError {
    ImageError(image::error::ImageError),
}

impl Display for TextureError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            TextureError::ImageError(e) => write!(f, "Error loading image: {}", e),
        }
    }
}

impl<'a> CreateTexture<TextureContext<'a>> for Texture {
    fn create<S: Into<[u32; 2]>>(
        TextureContext { device, queue }: &mut TextureContext<'a>,
        _format: Format,
        memory: &[u8],
        size: S,
        _settings: &TextureSettings, // TODO: Don't ignore settings
    ) -> Result<Self, TextureError> {
        let [width, height] = size.into();
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Diffuse Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            memory,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * width),
                rows_per_image: std::num::NonZeroU32::new(height),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Texture View"),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = Texture::create_bind_group_layout(device);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Self {
            texture,
            sampler,
            bind_group,
            width,
            height,
        })
    }
}

impl ImageSize for Texture {
    fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

pub struct Wgpu2d<'a> {
    device: &'a wgpu::Device,
    colored_render_pipelines: PsoBlend<wgpu::RenderPipeline>,
    textured_render_pipelines: PsoBlend<wgpu::RenderPipeline>,
}

impl<'a> Wgpu2d<'a> {
    pub fn new<'b>(device: &'a wgpu::Device, config: &'b wgpu::SurfaceConfiguration) -> Self {
        let colored_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Colored Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let colored_shader_module =
            device.create_shader_module(&wgpu::include_wgsl!("colored.wgsl"));

        let colored_render_pipelines = PsoBlend::new(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Colored Render Pipeline"),
                layout: Some(&colored_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &colored_shader_module,
                    entry_point: "main",
                    buffers: &[ColoredPipelineInput::desc()],
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &colored_shader_module,
                    entry_point: "main",
                    targets: &[wgpu::ColorTargetState {
                        format: config.format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    }],
                }),
            })
        });

        let textured_bind_group_layout = Texture::create_bind_group_layout(device);

        let textured_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Textured Pipeline Layout"),
                bind_group_layouts: &[&textured_bind_group_layout],
                push_constant_ranges: &[],
            });

        let textured_shader_module =
            device.create_shader_module(&wgpu::include_wgsl!("textured.wgsl"));

        let textured_render_pipelines = PsoBlend::new(|blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Textured Render Pipeline"),
                layout: Some(&textured_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &textured_shader_module,
                    entry_point: "main",
                    buffers: &[TexturedPipelineInput::desc()],
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &textured_shader_module,
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
            colored_render_pipelines,
            textured_render_pipelines,
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
    width: u32,
    height: u32,
    color_format: wgpu::TextureFormat,
    clear_color: Option<Color>,
    stencil: wgpu::Texture,
    stencil_view: wgpu::TextureView,
    render_bundles: Vec<(Option<[u32; 4]>, wgpu::RenderBundle)>,
}

impl<'a> WgpuGraphics<'a> {
    pub fn new(wgpu2d: &'a Wgpu2d<'a>, config: &wgpu::SurfaceConfiguration) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let stencil = wgpu2d.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Stencil Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        });
        let stencil_view = stencil.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Stencil Texture View"),
            ..Default::default()
        });
        Self {
            wgpu2d,
            width: config.width,
            height: config.height,
            color_format: config.format,
            clear_color: None,
            stencil,
            stencil_view,
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.stencil_view,
                    depth_ops: None,
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: true,
                    }),
                }),
            });

            render_pass.set_blend_constant(wgpu::Color::WHITE);

            for (scissor, render_bundle) in &self.render_bundles {
                let [x, y, width, height] = match scissor {
                    Some(rect) => *rect,
                    None => [0, 0, self.width, self.height],
                };
                render_pass.set_scissor_rect(x, y, width, height);
                render_pass.execute_bundles(std::iter::once(render_bundle));
            }
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
                depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_read_only: true,
                    stencil_read_only: false,
                }),
                sample_count: 1,
            });

        f(&mut render_bundle_encoder);

        render_bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Render Bundle"),
        })
    }

    fn bundle_colored(&mut self, colored_inputs: &[ColoredPipelineInput], draw_state: &DrawState) {
        let vertex_buffer =
            self.wgpu2d
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&colored_inputs),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let render_bundle = self.bundle(self.wgpu2d.device, |render_encoder| {
            render_encoder
                .set_pipeline(&self.wgpu2d.colored_render_pipelines.blend(draw_state.blend));
            render_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_encoder.draw(0..colored_inputs.len() as u32, 0..1);
        });

        self.render_bundles
            .push((draw_state.scissor, render_bundle));
    }

    fn bundle_textured(
        &mut self,
        textured_inputs: &[TexturedPipelineInput],
        texture: &Texture,
        draw_state: &DrawState,
    ) {
        let vertex_buffer =
            self.wgpu2d
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&textured_inputs),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let render_bundle = self.bundle(self.wgpu2d.device, |render_encoder| {
            render_encoder.set_pipeline(
                &self
                    .wgpu2d
                    .textured_render_pipelines
                    .blend(draw_state.blend),
            );
            render_encoder.set_bind_group(0, &texture.bind_group, &[]);
            render_encoder.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_encoder.draw(0..textured_inputs.len() as u32, 0..1);
        });

        self.render_bundles
            .push((draw_state.scissor, render_bundle));
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
                .map(|&position| ColoredPipelineInput { position, color })
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
                .map(|(&position, &color)| ColoredPipelineInput { position, color })
                .collect::<Vec<_>>();

            self.bundle_colored(&pipeline_inputs, draw_state);
        });
    }

    fn tri_list_uv<F>(
        &mut self,
        draw_state: &DrawState,
        &color: &[f32; 4],
        texture: &Texture,
        mut f: F,
    ) where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]], &[[f32; 2]])),
    {
        f(&mut |xys, uvs| {
            let pipeline_inputs = xys
                .iter()
                .zip(uvs.iter())
                .map(|(&xy, &uv)| TexturedPipelineInput { xy, uv, color })
                .collect::<Vec<_>>();

            self.bundle_textured(&pipeline_inputs, texture, draw_state);
        })
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
