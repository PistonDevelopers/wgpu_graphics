use graphics::{clear, rectangle};
use piston::{
    ButtonEvent, ButtonState, EventSettings, Events, RenderEvent, ResizeArgs, ResizeEvent, Window,
    WindowSettings,
};
use winit_window::WinitWindow;

fn main() {
    let mut window = WinitWindow::new(&WindowSettings::new("wgpu_graphics example", (640, 480)));

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(window.get_window()) };
    let adapter =
        futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .unwrap();

    let (device, queue) = futures::executor::block_on(
        adapter.request_device(&wgpu::DeviceDescriptor::default(), None),
    )
    .unwrap();
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_preferred_format(&adapter).unwrap(),
        width: window.draw_size().width as u32,
        height: window.draw_size().height as u32,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &surface_config);

    let mut wgpu2d = wgpu_graphics::Wgpu2d::new(&device, &surface_config);
    let mut events = Events::new(EventSettings::new());

    let colors = [
        [0.0, 0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0, 1.0],
        [1.0, 0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0, 1.0],
    ];
    let mut i = 0;
    while let Some(event) = events.next(&mut window) {
        event.resize(
            |&ResizeArgs {
                 draw_size: [width, height],
                 ..
             }| {
                surface_config = wgpu::SurfaceConfiguration {
                    width,
                    height,
                    ..surface_config
                };
                surface.configure(&device, &surface_config);
            },
        );
        event.render(|render_args| {
            let surface_texture = &surface.get_current_frame().unwrap().output.texture;
            let surface_view = surface_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let command_buffer = wgpu2d.draw(
                &device,
                &surface_config,
                &surface_view,
                render_args.viewport(),
                |c, g| {
                    clear(colors[i], g);

                    rectangle(
                        colors[(i + 1) % colors.len()],
                        [0.0, 0.0, 100.0, 100.0],
                        c.transform,
                        g,
                    );
                },
            );
            queue.submit(std::iter::once(command_buffer));
        });
        event.button(|button_args| {
            if button_args.state == ButtonState::Press {
                i = (i + 1) % colors.len();
            }
        });
    }
}
