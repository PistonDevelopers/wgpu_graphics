use graphics::{clear, DrawState, Text, Transformed};
use piston::{EventSettings, Events, RenderEvent, ResizeArgs, ResizeEvent, Window, WindowSettings};
use texture::TextureSettings;
use wgpu_graphics::{GlyphCache, TextureContext};
use winit_window::WinitWindow;

fn main() {
    let mut window = WinitWindow::new(&WindowSettings::new("wgpu_graphics: text_test", (500, 300)));

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

    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();
    let texture_context = TextureContext::from_parts(&device, &queue);
    let mut glyph_cache = GlyphCache::new(
        assets.join("FiraSans-Regular.ttf"),
        texture_context,
        TextureSettings::new(),
    )
    .unwrap();

    let mut wgpu2d = wgpu_graphics::Wgpu2d::new(&device, &surface_config);
    let mut events = Events::new(EventSettings::new());

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
                    clear([1.0; 4], g);
                    Text::new_color([0.0, 0.5, 0.0, 1.0], 32)
                        .draw(
                            "Hello wgpu_graphics!",
                            &mut glyph_cache,
                            &DrawState::default(),
                            c.transform.trans(10.0, 100.0),
                            g,
                        )
                        .unwrap();
                },
            );
            queue.submit(std::iter::once(command_buffer));
        });
    }
}
