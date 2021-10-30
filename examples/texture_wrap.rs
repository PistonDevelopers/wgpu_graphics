mod include;

use crate::include::{event_resize, init_surface_config};
use graphics::{clear, DrawState, Graphics};
use piston::{Button, EventSettings, Events, Key, PressEvent, RenderEvent, WindowSettings};
use texture::{TextureSettings, Wrap};
use wgpu_graphics::{Texture, TextureContext};
use winit_window::WinitWindow;

fn main() {
    println!("Press U to change the texture wrap mode for the u coordinate");
    println!("Press V to change the texture wrap mode for the v coordinate");

    let mut window = WinitWindow::new(&WindowSettings::new(
        "wgpu_graphics: texture_wrap",
        (640, 480),
    ));

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(window.get_window()) };
    let adapter =
        futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .unwrap();

    let (device, queue) = futures::executor::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Device"),
            features: wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
            ..Default::default()
        },
        None,
    ))
    .unwrap();
    let mut surface_config = init_surface_config(&surface, &adapter, &window);
    surface.configure(&device, &surface_config);

    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();
    let wrap_modes = [
        Wrap::ClampToEdge,
        Wrap::ClampToBorder,
        Wrap::Repeat,
        Wrap::MirroredRepeat,
    ];
    let mut ix_u = 0;
    let mut ix_v = 0;
    let mut texture_settings = TextureSettings::new();
    texture_settings.set_border_color([0.0, 0.0, 0.0, 1.0]);

    let mut texture_context = TextureContext::from_parts(&device, &queue);
    let mut rust_logo = Texture::from_path(
        &mut texture_context,
        assets.join("rust.png"),
        &texture_settings,
    )
    .unwrap();

    let mut wgpu2d = wgpu_graphics::Wgpu2d::new(&device, &surface_config);
    let mut events = Events::new(EventSettings::new());

    while let Some(event) = events.next(&mut window) {
        event_resize(&event, &device, &surface, &mut surface_config);
        event.render(|render_args| {
            let surface_texture = surface.get_current_texture().unwrap();
            let surface_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

            let command_buffer = wgpu2d.draw(
                &device,
                &surface_config,
                &surface_view,
                render_args.viewport(),
                |_, g| {
                    clear([1.0; 4], g);
                    let points = [[0.5, 0.5], [-0.5, 0.5], [-0.5, -0.5], [0.5, -0.5]];
                    // (0, 1, 2) and (0, 2, 3)
                    let uvs = [
                        [4.0, 0.0],
                        [0.0, 0.0],
                        [0.0, 4.0],
                        [4.0, 0.0],
                        [0.0, 4.0],
                        [4.0, 4.0],
                    ];
                    let mut verts = [[0.0, 0.0]; 6];
                    let indices_points: [usize; 6] = [0, 1, 2, 0, 2, 3];
                    for (ixv, &ixp) in (0..6).zip(indices_points.iter()) {
                        verts[ixv] = points[ixp];
                    }
                    g.tri_list_uv(&DrawState::new_alpha(), &[1.0; 4], &rust_logo, |f| {
                        f(&verts, &uvs)
                    });
                },
            );
            queue.submit(std::iter::once(command_buffer));
            surface_texture.present();
        });

        if let Some(Button::Keyboard(Key::U)) = event.press_args() {
            ix_u = (ix_u + 1) % wrap_modes.len();
            texture_settings.set_wrap_u(wrap_modes[ix_u]);
            rust_logo = Texture::from_path(
                &mut texture_context,
                assets.join("rust.png"),
                &texture_settings,
            )
            .unwrap();
            println!(
                "Changed texture wrap mode for u coordinate to: {:?}",
                wrap_modes[ix_u]
            );
        }
        if let Some(Button::Keyboard(Key::V)) = event.press_args() {
            ix_v = (ix_v + 1) % wrap_modes.len();
            texture_settings.set_wrap_v(wrap_modes[ix_v]);
            rust_logo = Texture::from_path(
                &mut texture_context,
                assets.join("rust.png"),
                &texture_settings,
            )
            .unwrap();
            println!(
                "Changed texture wrap mode for v coordinate to: {:?}",
                wrap_modes[ix_v]
            );
        }
    }
}
