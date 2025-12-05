mod include;

use crate::include::{event_resize, init_surface_config};
use graphics::{clear, Rectangle};
use piston::{EventSettings, Events, RenderEvent, WindowSettings};
use winit_window::WinitWindow;
use std::sync::Arc;

fn main() {
    let settings = WindowSettings::new("wgpu_graphics example", (300, 300))
        .exit_on_esc(true);
    let mut window = WinitWindow::new(&settings);

    let instance = wgpu::Instance::new(&Default::default());
    let surface = instance.create_surface(window.get_window()).unwrap();
    let adapter =
        futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .unwrap();

    let mut device_descriptor = wgpu::DeviceDescriptor::default();
    device_descriptor.required_features.set(wgpu::Features::DEPTH_CLIP_CONTROL, true);
    let (device, queue) = futures::executor::block_on(
        adapter.request_device(&device_descriptor),
    )
    .unwrap();
    let mut surface_config = init_surface_config(&surface, &adapter, &window);
    surface.configure(&device, &surface_config);

    let device = Arc::new(device);
    let mut wgpu2d = wgpu_graphics::Wgpu2d::new(device.clone(), &surface_config);
    let mut events = Events::new(EventSettings::new());

    while let Some(event) = events.next(&mut window) {
        event_resize(&event, &device, &surface, &mut surface_config);
        event.render(|render_args| {
            let surface_texture = surface.get_current_texture().unwrap();
            let surface_view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let ((), command_buffer) = wgpu2d.draw(
                &surface_config,
                &surface_view,
                render_args.viewport(),
                |c, g| {
                    clear([1.0; 4], g);
                    Rectangle::new([1.0, 0.0, 0.0, 1.0]).draw(
                        [10.0, 10.0, 100.0, 100.0],
                        &c.draw_state,
                        c.transform,
                        g,
                    );
                },
            );
            queue.submit(std::iter::once(command_buffer));
            surface_texture.present();
        });
    }
}
