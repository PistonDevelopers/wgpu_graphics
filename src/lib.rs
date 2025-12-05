//! A [Piston 2D graphics](https://github.com/pistondevelopers/graphics) back-end using [wgpu](https://github.com/gfx-rs/wgpu).

use graphics::{
    draw_state::{Blend, Stencil},
    types::Color,
    Context, DrawState, Graphics, Viewport,
};
use std::{
    fmt::{self, Display, Formatter},
    path::Path,
    sync::Arc,
};
use wgpu::util::DeviceExt;
use wgpu::StoreOp;

pub use graphics::ImageSize;
pub use texture::*;

/// Stores textures for text rendering.
pub type GlyphCache<'a> =
    graphics::glyph_cache::rusttype::GlyphCache<'a, TextureContext<'a>, Texture>;

/// Input struct for the "colored" pipeline's vertex shader.
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

/// Input struct for the "textured" pipeline's vertex shader.
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

/// Stores `T` object for each Blend mode.
struct PsoBlend<T> {
    none: T,
    alpha: T,
    add: T,
    lighter: T,
    multiply: T,
    invert: T,
}

impl<T> PsoBlend<T> {
    /// Returns `T` object for `blend`.
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

/// Stores `T` object for each (Stencil, Blend) mode.
struct PsoStencil<T> {
    none: PsoBlend<T>,
    clip: PsoBlend<T>,
    inside: PsoBlend<T>,
    outside: PsoBlend<T>,
    increment: PsoBlend<T>,
}

impl<T> PsoStencil<T> {
    /// Creates a new `PsoStencil<T>`, using `f`, for all (Stencil, Blend) mode.
    fn new<F>(mut f: F) -> PsoStencil<T>
    where
        F: FnMut(Option<wgpu::BlendState>, wgpu::StencilState) -> T,
    {
        use wgpu::{
            BlendComponent, BlendFactor, BlendOperation, BlendState, CompareFunction,
            StencilFaceState, StencilOperation, StencilState,
        };

        let stencil_none = StencilState {
            front: StencilFaceState::IGNORE,
            back: StencilFaceState::IGNORE,
            read_mask: 0,
            write_mask: 0,
        };
        let stencil_clip = StencilState {
            front: StencilFaceState {
                compare: CompareFunction::Never,
                fail_op: StencilOperation::Replace,
                ..Default::default()
            },
            back: StencilFaceState {
                compare: CompareFunction::Never,
                fail_op: StencilOperation::Replace,
                ..Default::default()
            },
            read_mask: 255,
            write_mask: 255,
        };
        let stencil_inside = StencilState {
            front: StencilFaceState {
                compare: CompareFunction::Equal,
                ..Default::default()
            },
            back: StencilFaceState {
                compare: CompareFunction::Equal,
                ..Default::default()
            },
            read_mask: 255,
            write_mask: 255,
        };
        let stencil_outside = StencilState {
            front: StencilFaceState {
                compare: CompareFunction::NotEqual,
                ..Default::default()
            },
            back: StencilFaceState {
                compare: CompareFunction::NotEqual,
                ..Default::default()
            },
            read_mask: 255,
            write_mask: 255,
        };
        let stencil_increment = StencilState {
            front: StencilFaceState {
                compare: CompareFunction::Never,
                fail_op: StencilOperation::IncrementClamp,
                ..Default::default()
            },
            back: StencilFaceState {
                compare: CompareFunction::Never,
                fail_op: StencilOperation::IncrementClamp,
                ..Default::default()
            },
            read_mask: 255,
            write_mask: 255,
        };

        let blend_add = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };
        let blend_lighter = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };
        let blend_multiply = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Dst,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::DstAlpha,
                dst_factor: BlendFactor::Zero,
                operation: BlendOperation::Add,
            },
        };
        let blend_invert = BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::Constant,
                dst_factor: BlendFactor::Src,
                operation: BlendOperation::Subtract,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::Zero,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };

        PsoStencil {
            none: PsoBlend {
                none: f(None, stencil_none.clone()),
                alpha: f(Some(BlendState::ALPHA_BLENDING), stencil_none.clone()),
                add: f(Some(blend_add), stencil_none.clone()),
                lighter: f(Some(blend_lighter), stencil_none.clone()),
                multiply: f(Some(blend_multiply), stencil_none.clone()),
                invert: f(Some(blend_invert), stencil_none),
            },
            clip: PsoBlend {
                none: f(None, stencil_clip.clone()),
                alpha: f(Some(BlendState::ALPHA_BLENDING), stencil_clip.clone()),
                add: f(Some(blend_add), stencil_clip.clone()),
                lighter: f(Some(blend_lighter), stencil_clip.clone()),
                multiply: f(Some(blend_multiply), stencil_clip.clone()),
                invert: f(Some(blend_invert), stencil_clip),
            },
            inside: PsoBlend {
                none: f(None, stencil_inside.clone()),
                alpha: f(Some(BlendState::ALPHA_BLENDING), stencil_inside.clone()),
                add: f(Some(blend_add), stencil_inside.clone()),
                lighter: f(Some(blend_lighter), stencil_inside.clone()),
                multiply: f(Some(blend_multiply), stencil_inside.clone()),
                invert: f(Some(blend_invert), stencil_inside),
            },
            outside: PsoBlend {
                none: f(None, stencil_outside.clone()),
                alpha: f(Some(BlendState::ALPHA_BLENDING), stencil_outside.clone()),
                add: f(Some(blend_add), stencil_outside.clone()),
                lighter: f(Some(blend_lighter), stencil_outside.clone()),
                multiply: f(Some(blend_multiply), stencil_outside.clone()),
                invert: f(Some(blend_invert), stencil_outside),
            },
            increment: PsoBlend {
                none: f(None, stencil_increment.clone()),
                alpha: f(Some(BlendState::ALPHA_BLENDING), stencil_increment.clone()),
                add: f(Some(blend_add), stencil_increment.clone()),
                lighter: f(Some(blend_lighter), stencil_increment.clone()),
                multiply: f(Some(blend_multiply), stencil_increment.clone()),
                invert: f(Some(blend_invert), stencil_increment),
            },
        }
    }

    /// Returns `T` object for `stencil` and `blend`.
    fn stencil_blend(&self, stencil: Option<Stencil>, blend: Option<Blend>) -> (&T, Option<u8>) {
        match stencil {
            None => (self.none.blend(blend), None),
            Some(Stencil::Clip(val)) => (self.clip.blend(blend), Some(val)),
            Some(Stencil::Inside(val)) => (self.inside.blend(blend), Some(val)),
            Some(Stencil::Outside(val)) => (self.outside.blend(blend), Some(val)),
            Some(Stencil::Increment) => (self.increment.blend(blend), None),
        }
    }
}

/// Represents a texture.
#[derive(Clone, PartialEq, Eq)]
pub struct Texture {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    width: u32,
    height: u32,
}

/// Context required to create and update textures.
pub struct TextureContext<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl<'a> TextureContext<'a> {
    /// Creates a new `TextureContext` from its parts.
    pub fn from_parts(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        TextureContext { device, queue }
    }
}

impl Texture {
    /// Creates a `Texture` with image loading from `path`.
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

    /// Creates a `Texture` with `img`.
    pub fn from_image<'a>(
        context: &mut TextureContext<'a>,
        img: &image::RgbaImage,
        settings: &TextureSettings,
    ) -> Result<Self, TextureError> {
        let (width, height) = img.dimensions();
        CreateTexture::create(context, Format::Rgba8, img, [width, height], settings)
    }

    /// Creates a [`BindGroupLayout`](`wgpu::BindGroupLayout`) for "textured" pipeline's fragment shader's binding.
    // FIXME: Maybe should be moved out of `impl Texture`?
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}

impl<'a> TextureOp<TextureContext<'a>> for Texture {
    type Error = TextureError;
}

/// Texture creation or update error.
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

#[allow(clippy::float_cmp)]
impl<'a> CreateTexture<TextureContext<'a>> for Texture {
    fn create<S: Into<[u32; 2]>>(
        TextureContext { device, queue }: &mut TextureContext<'a>,
        _format: Format,
        memory: &[u8],
        size: S,
        settings: &TextureSettings,
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
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            memory,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Texture View"),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: match settings.get_wrap_u() {
                Wrap::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                Wrap::Repeat => wgpu::AddressMode::Repeat,
                Wrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                Wrap::ClampToBorder => wgpu::AddressMode::ClampToBorder,
            },
            address_mode_v: match settings.get_wrap_v() {
                Wrap::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                Wrap::Repeat => wgpu::AddressMode::Repeat,
                Wrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                Wrap::ClampToBorder => wgpu::AddressMode::ClampToBorder,
            },
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: match settings.get_mag() {
                Filter::Linear => wgpu::FilterMode::Linear,
                Filter::Nearest => wgpu::FilterMode::Nearest,
            },
            min_filter: match settings.get_min() {
                Filter::Linear => wgpu::FilterMode::Linear,
                Filter::Nearest => wgpu::FilterMode::Nearest,
            },
            mipmap_filter: match settings.get_mipmap() {
                Filter::Linear => wgpu::FilterMode::Linear,
                Filter::Nearest => wgpu::FilterMode::Nearest,
            },
            border_color: if settings.get_border_color() == [0.0; 4] {
                Some(wgpu::SamplerBorderColor::TransparentBlack)
            } else if settings.get_border_color() == [0.0, 0.0, 0.0, 1.0] {
                Some(wgpu::SamplerBorderColor::OpaqueBlack)
            } else if settings.get_border_color() == [1.0; 4] {
                Some(wgpu::SamplerBorderColor::OpaqueWhite)
            } else {
                None
            },
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
            bind_group,
            width,
            height,
        })
    }
}

impl<'a> UpdateTexture<TextureContext<'a>> for Texture {
    fn update<O, S>(
        &mut self,
        TextureContext { queue, .. }: &mut TextureContext<'a>,
        _format: Format,
        memory: &[u8],
        offset: O,
        size: S,
    ) -> Result<(), TextureError>
    where
        O: Into<[u32; 2]>,
        S: Into<[u32; 2]>,
    {
        let Texture { ref texture, .. } = self;
        let [x, y] = offset.into();
        let [width, height] = size.into();

        let origin = wgpu::Origin3d { x, y, z: 0 };
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture,
                mip_level: 0,
                origin,
                aspect: wgpu::TextureAspect::All,
            },
            memory,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        Ok(())
    }
}

impl ImageSize for Texture {
    fn get_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

use graphics::BACK_END_MAX_VERTEX_COUNT as BUFFER_SIZE;
// The number of chunks to fill up before rendering.
// Amount of memory used: `BUFFER_SIZE * CHUNKS * 4 * (2 + 4)`
// `4` for bytes per f32, and `2 + 4` for position and color.
const CHUNKS: usize = 100;
const SOFT_BUFFER_LIMIT: usize = CHUNKS * BUFFER_SIZE;

/// The resource needed for rendering 2D.
pub struct Wgpu2d {
    device: Arc<wgpu::Device>,
    colored_render_pipelines: PsoStencil<wgpu::RenderPipeline>,
    textured_render_pipelines: PsoStencil<wgpu::RenderPipeline>,
    colored_data: Vec<ColoredPipelineInput>,
    textured_data: Vec<TexturedPipelineInput>,
}

impl Wgpu2d {
    /// Creates a new `Wgpu2d`.
    pub fn new<'b>(
        device: Arc<wgpu::Device>,
        config: &'b wgpu::SurfaceConfiguration,
    ) -> Self {
        let colored_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Colored Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let colored_shader_module =
            device.create_shader_module(wgpu::include_wgsl!("colored.wgsl"));

        let colored_render_pipelines = PsoStencil::new(|blend, stencil| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Colored Render Pipeline"),
                layout: Some(&colored_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &colored_shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[ColoredPipelineInput::desc()],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: true,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil,
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &colored_shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
            })
        });

        let textured_bind_group_layout = Texture::create_bind_group_layout(&device);

        let textured_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Textured Pipeline Layout"),
                bind_group_layouts: &[&textured_bind_group_layout],
                push_constant_ranges: &[],
            });

        let textured_shader_module =
            device.create_shader_module(wgpu::include_wgsl!("textured.wgsl"));

        let textured_render_pipelines = PsoStencil::new(|blend, stencil| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Textured Render Pipeline"),
                layout: Some(&textured_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &textured_shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[TexturedPipelineInput::desc()],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: true,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24PlusStencil8,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil,
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &textured_shader_module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
            })
        });

        Self {
            device,
            colored_render_pipelines,
            textured_render_pipelines,
            colored_data: Vec::with_capacity(SOFT_BUFFER_LIMIT),
            textured_data: Vec::with_capacity(SOFT_BUFFER_LIMIT),
        }
    }

    /// Performs 2D graphics operations and returns encoded commands.
    ///
    /// To actually draw on a window surface, you must [`submit`](`wgpu::Queue::submit`) the returned [`CommandBuffer`](`wgpu::CommandBuffer`).
    pub fn draw<F, U>(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        output_view: &wgpu::TextureView,
        viewport: Viewport,
        f: F,
    ) -> (U, wgpu::CommandBuffer)
    where
        F: FnOnce(Context, &mut WgpuGraphics) -> U,
    {
        let mut g = WgpuGraphics::new(self, config, output_view);
        let c = Context::new_viewport(viewport);
        let res = f(c, &mut g);
        (res, g.draw())
    }
}

/// Graphics back-end.
pub struct WgpuGraphics<'a> {
    wgpu2d: &'a mut Wgpu2d,
    width: u32,
    height: u32,
    stencil_view: wgpu::TextureView,
    command_encoder: wgpu::CommandEncoder,
    output_view: &'a wgpu::TextureView,
    draw_state: DrawState,
    texture: Option<Texture>,
}

impl<'a> WgpuGraphics<'a> {
    /// Creates a new `WgpuGraphics`.
    pub fn new(
        wgpu2d: &'a mut Wgpu2d,
        config: &wgpu::SurfaceConfiguration,
        output_view: &'a wgpu::TextureView,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let device = &wgpu2d.device;
        let stencil = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Stencil Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Depth24PlusStencil8],
        });
        let stencil_view = stencil.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Stencil Texture View"),
            ..Default::default()
        });
        let command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });
        Self {
            wgpu2d,
            width: config.width,
            height: config.height,
            stencil_view,
            command_encoder,
            output_view,
            draw_state: DrawState::default(),
            texture: None,
        }
    }

    /// Performs 2D graphics operations and returns encoded commands.
    ///
    /// To actually draw on a window surface, you must [`submit`](`wgpu::Queue::submit`) the returned [`CommandBuffer`](`wgpu::CommandBuffer`).
    pub fn draw(mut self) -> wgpu::CommandBuffer {
        if self.wgpu2d.colored_data.len() > 0 {
            self.command_colored();
        }
        if self.wgpu2d.textured_data.len() > 0 {
            self.command_textured();
        }

        self.command_encoder.finish()
    }

    fn command_colored(&mut self) {
        let draw_state = &self.draw_state;
        let colored_inputs = &*self.wgpu2d.colored_data;
        let output_view = self.output_view;
        let encoder = &mut self.command_encoder;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Colored Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                }),
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_blend_constant(wgpu::Color::WHITE);

        let vertex_buffer =
            self.wgpu2d
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(colored_inputs),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let (pipeline, stencil_val) = self
            .wgpu2d
            .colored_render_pipelines
            .stencil_blend(draw_state.stencil, draw_state.blend);

        let [x, y, width, height] = match draw_state.scissor {
            Some(rect) => rect,
            None => [0, 0, self.width, self.height],
        };
        render_pass.set_pipeline(pipeline);
        render_pass.set_scissor_rect(x, y, width, height);
        if let Some(stencil_val) = stencil_val {
            render_pass.set_stencil_reference(stencil_val as u32);
        }

        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..colored_inputs.len() as u32, 0..1);

        self.wgpu2d.colored_data.clear();
    }

    fn command_textured(&mut self) {
        let texture = &self.texture.as_ref().unwrap();
        let draw_state = &self.draw_state;
        let textured_inputs = &*self.wgpu2d.textured_data;
        let output_view = self.output_view;
        let encoder = &mut self.command_encoder;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Colored Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                }),
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_blend_constant(wgpu::Color::WHITE);

        let vertex_buffer =
            self.wgpu2d
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(textured_inputs),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        let (pipeline, stencil_val) = self
            .wgpu2d
            .textured_render_pipelines
            .stencil_blend(draw_state.stencil, draw_state.blend);

        let [x, y, width, height] = match draw_state.scissor {
            Some(rect) => rect,
            None => [0, 0, self.width, self.height],
        };
        render_pass.set_pipeline(pipeline);
        render_pass.set_scissor_rect(x, y, width, height);
        if let Some(stencil_val) = stencil_val {
            render_pass.set_stencil_reference(stencil_val as u32);
        }

        render_pass.set_bind_group(0, Some(&texture.bind_group), &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..textured_inputs.len() as u32, 0..1);

        self.wgpu2d.textured_data.clear();
    }
}

impl<'a> Graphics for WgpuGraphics<'a> {
    type Texture = Texture;

    fn clear_color(&mut self, color: Color) {
        if self.wgpu2d.colored_data.len() > 0 {
            self.command_colored();
        }
        if self.wgpu2d.textured_data.len() > 0 {
            self.command_textured();
        }

        let output_view = self.output_view;
        let color_load = wgpu::LoadOp::Clear(to_wgpu_color(color));
        let encoder = &mut self.command_encoder;
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Color Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: color_load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                }),
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }

    fn clear_stencil(&mut self, value: u8) {
        if self.wgpu2d.colored_data.len() > 0 {
            self.command_colored();
        }
        if self.wgpu2d.textured_data.len() > 0 {
            self.command_textured();
        }

        let output_view = self.output_view;
        let stencil_load = wgpu::LoadOp::Clear(value as u32);
        let encoder = &mut self.command_encoder;
        let _ = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear Stencil Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                depth_slice: None,
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.stencil_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: stencil_load,
                    store: StoreOp::Store,
                }),
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }

    fn tri_list<F>(&mut self, draw_state: &DrawState, &color: &[f32; 4], mut f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]])),
    {
        if self.wgpu2d.colored_data.len() > 0 {
            let flush = self.wgpu2d.colored_data.len() + BUFFER_SIZE >= SOFT_BUFFER_LIMIT ||
                draw_state != &self.draw_state;
            if flush {self.command_colored()}
        }
        if self.wgpu2d.textured_data.len() > 0 {
            self.command_textured();
        }

        self.draw_state = *draw_state;
        f(&mut |positions| {
            if self.wgpu2d.colored_data.len() + BUFFER_SIZE >= SOFT_BUFFER_LIMIT {
                self.command_colored();
            }
            self.wgpu2d.colored_data.extend(positions
                .iter()
                .map(|&position| ColoredPipelineInput { position, color }));
        })
    }

    fn tri_list_c<F>(&mut self, draw_state: &DrawState, mut f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]], &[[f32; 4]])),
    {
        if self.wgpu2d.colored_data.len() > 0 {
            let flush = self.wgpu2d.colored_data.len() + BUFFER_SIZE >= SOFT_BUFFER_LIMIT ||
                draw_state != &self.draw_state;
            if flush {self.command_colored()}
        }
        if self.wgpu2d.textured_data.len() > 0 {
            self.command_textured();
        }

        self.draw_state = *draw_state;
        f(&mut |positions, colors| {
            if self.wgpu2d.colored_data.len() + BUFFER_SIZE >= SOFT_BUFFER_LIMIT {
                self.command_colored();
            }
            self.wgpu2d.colored_data.extend(positions
                .iter()
                .zip(colors.iter())
                .map(|(&position, &color)| ColoredPipelineInput { position, color }));
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
        if self.wgpu2d.colored_data.len() > 0 {
            self.command_colored();
        }
        if self.wgpu2d.textured_data.len() > 0 {
            let flush = self.wgpu2d.textured_data.len() + BUFFER_SIZE >= SOFT_BUFFER_LIMIT ||
                draw_state != &self.draw_state;
            if flush {self.command_textured()}
            else if let Some(prev_texture) = self.texture.as_ref() {
                if texture != prev_texture {
                    self.command_textured();
                }
            }
        }

        self.texture = Some(texture.clone());
        self.draw_state = *draw_state;
        f(&mut |xys, uvs| {
            if self.wgpu2d.textured_data.len() + BUFFER_SIZE >= SOFT_BUFFER_LIMIT {
                self.command_textured();
            }
            self.wgpu2d.textured_data.extend(xys
                .iter()
                .zip(uvs.iter())
                .map(|(&xy, &uv)| TexturedPipelineInput { xy, uv, color }));
        })
    }

    fn tri_list_uv_c<F>(&mut self, draw_state: &DrawState, texture: &Texture, mut f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]], &[[f32; 2]], &[[f32; 4]])),
    {
        if self.wgpu2d.colored_data.len() > 0 {
            self.command_colored();
        }
        if self.wgpu2d.textured_data.len() > 0 {
            let flush = self.wgpu2d.textured_data.len() + BUFFER_SIZE >= SOFT_BUFFER_LIMIT ||
                draw_state != &self.draw_state;
            if flush {self.command_textured()}
            else if let Some(prev_texture) = self.texture.as_ref() {
                if texture != prev_texture {
                    self.command_textured();
                }
            }
        }

        self.texture = Some(texture.clone());
        self.draw_state = *draw_state;
        f(&mut |xys, uvs, colors| {
            if self.wgpu2d.textured_data.len() + BUFFER_SIZE >= SOFT_BUFFER_LIMIT {
                self.command_textured();
            }
            self.wgpu2d.textured_data.extend(xys
                .iter()
                .zip(uvs.iter())
                .zip(colors.iter())
                .map(|((&xy, &uv), &color)| TexturedPipelineInput { xy, uv, color }));
        })
    }
}

fn to_wgpu_color(color: Color) -> wgpu::Color {
    wgpu::Color {
        r: color[0] as f64,
        g: color[1] as f64,
        b: color[2] as f64,
        a: color[3] as f64,
    }
}
