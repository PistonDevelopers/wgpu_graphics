use piston::{Event, ResizeArgs, ResizeEvent, Window};
use wgpu::{Adapter, Device, PresentMode, Surface, SurfaceConfiguration, TextureFormat};
use winit_window::WinitWindow;

pub fn init_surface_config(
    _surface: &Surface,
    _adapter: &Adapter,
    window: &WinitWindow,
) -> SurfaceConfiguration {
    SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: TextureFormat::Bgra8UnormSrgb,
        width: window.draw_size().width as u32,
        height: window.draw_size().height as u32,
        present_mode: PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
    }
}

pub fn event_resize(
    event: &Event,
    device: &Device,
    surface: &Surface,
    surface_config: &mut SurfaceConfiguration,
) {
    event.resize(
        |&ResizeArgs {
             draw_size: [width, height],
             ..
         }| {
            *surface_config = SurfaceConfiguration {
                width,
                height,
                ..*surface_config
            };
            surface.configure(device, surface_config);
        },
    );
}
