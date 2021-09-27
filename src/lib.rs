use graphics::{types::Color, DrawState, Graphics, ImageSize};

pub struct Texture {}

impl ImageSize for Texture {
    fn get_size(&self) -> (u32, u32) {
        todo!()
    }
}

pub struct WgpuGraphics {
    clear_color: Option<Color>,
}

impl WgpuGraphics {
    pub fn new() -> Self {
        Self { clear_color: None }
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
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load, store: true },
                }],
                depth_stencil_attachment: None,
            });
        })
    }
}

impl Graphics for WgpuGraphics {
    type Texture = Texture;

    fn clear_color(&mut self, color: Color) {
        self.clear_color = Some(color);
    }

    fn clear_stencil(&mut self, value: u8) {}

    fn tri_list<F>(&mut self, draw_state: &DrawState, color: &[f32; 4], f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]])),
    {
        todo!()
    }

    fn tri_list_c<F>(&mut self, draw_state: &DrawState, f: F)
    where
        F: FnMut(&mut dyn FnMut(&[[f32; 2]], &[[f32; 4]])),
    {
        todo!()
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

pub fn draw<F>(device: &wgpu::Device, output_view: &wgpu::TextureView, f: F) -> wgpu::CommandBuffer
where
    F: FnOnce(&mut WgpuGraphics),
{
    let mut g = WgpuGraphics::new();
    f(&mut g);
    g.draw(device, output_view)
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
