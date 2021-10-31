mod include;

use crate::include::{event_resize, init_surface_config};
use graphics::{clear, Graphics, Rectangle, Transformed};
use piston::{EventSettings, Events, RenderEvent, WindowSettings};
use texture::TextureSettings;
use wgpu_graphics::{Texture, TextureContext};
use winit_window::WinitWindow;

fn main() {
    let mut window = WinitWindow::new(&WindowSettings::new("wgpu_graphics example", (300, 300)));

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
    let mut surface_config = init_surface_config(&surface, &adapter, &window);
    surface.configure(&device, &surface_config);

    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();
    let mut texture_context = TextureContext::from_parts(&device, &queue);
    let rust_logo = Texture::from_path(
        &mut texture_context,
        assets.join("rust-white.png"),
        &TextureSettings::new(),
    )
    .unwrap();

    let mut wgpu2d = wgpu_graphics::Wgpu2d::new(&device, &surface_config);
    let mut events = Events::new(EventSettings::new());

    while let Some(event) = events.next(&mut window) {
        event_resize(&event, &device, &surface, &mut surface_config);
        event.render(|render_args| {
            let surface_texture = surface.get_current_texture().unwrap();
            let surface_view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let command_buffer = wgpu2d.draw(
                &device,
                &surface_config,
                &surface_view,
                render_args.viewport(),
                |c, g| {
                    use graphics::triangulation::{tx, ty};

                    let transform = c.transform.trans(0.0, 0.0);
                    let tr = |p: [f64; 2]| [tx(transform, p[0], p[1]), ty(transform, p[0], p[1])];

                    clear([1.0; 4], g);
                    Rectangle::new([1.0, 0.0, 0.0, 1.0]).draw(
                        [0.0, 0.0, 100.0, 100.0],
                        &c.draw_state,
                        c.transform,
                        g,
                    );
                    Rectangle::new([0.0, 1.0, 0.0, 0.3]).draw(
                        [50.0, 50.0, 100.0, 100.0],
                        &c.draw_state,
                        c.transform,
                        g,
                    );
                    g.tri_list_uv_c(&c.draw_state, &rust_logo, |f| {
                        (f)(
                            &[
                                tr([0.0, 0.0]),
                                tr([300.0, 0.0]),
                                tr([0.0, 300.0]),
                                tr([300.0, 0.0]),
                                tr([0.0, 300.0]),
                                tr([300.0, 300.0]),
                            ],
                            &[
                                [0.0, 0.0],
                                [1.0, 0.0],
                                [0.0, 1.0],
                                [1.0, 0.0],
                                [0.0, 1.0],
                                [1.0, 1.0],
                            ],
                            &[
                                [1.0, 0.0, 0.0, 1.0],
                                [0.0, 1.0, 0.0, 1.0],
                                [0.0, 0.0, 1.0, 1.0],
                                [0.0, 00.0, 0.0, 1.0],
                                [0.0, 00.0, 0.0, 1.0],
                                [0.0, 00.0, 0.0, 1.0],
                            ],
                        )
                    });
                },
            );
            queue.submit(std::iter::once(command_buffer));
            surface_texture.present();
        });
    }
}
