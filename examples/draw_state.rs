mod include;

use crate::include::{event_resize, init_surface_config};
use graphics::{clear, draw_state::Blend, DrawState, Ellipse, Image, Rectangle, Transformed};
use piston::{Button, EventSettings, Events, Key, PressEvent, RenderEvent, WindowSettings};
use texture::TextureSettings;
use wgpu_graphics::{Texture, TextureContext};
use winit_window::WinitWindow;
use std::sync::Arc;

fn main() {
    println!("Press A to change blending");
    println!("Press S to change clip inside/out");

    let settings = WindowSettings::new("wgpu_graphics example", (640, 480))
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
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    let mut surface_config = init_surface_config(&surface, &adapter, &window);
    surface.configure(&device, &surface_config);

    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();
    let blends = [
        Blend::Alpha,
        Blend::Add,
        Blend::Invert,
        Blend::Multiply,
        Blend::Lighter,
    ];
    let mut blend = 0;
    let mut clip_inside = true;
    let mut texture_context = TextureContext::from_parts(device.clone(), queue.clone());
    let rust_logo = Texture::from_path(
        &mut texture_context,
        assets.join("rust.png"),
        &TextureSettings::new(),
    )
    .unwrap();

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
                    clear([0.8, 0.8, 0.8, 1.0], g);
                    Rectangle::new([1.0, 0.0, 0.0, 1.0]).draw(
                        [0.0, 0.0, 100.0, 100.0],
                        &c.draw_state,
                        c.transform,
                        g,
                    );

                    let draw_state = c.draw_state.blend(blends[blend]);
                    Rectangle::new([0.5, 1.0, 0.0, 0.3]).draw(
                        [50.0, 50.0, 100.0, 100.0],
                        &draw_state,
                        c.transform,
                        g,
                    );

                    let transform = c.transform.trans(100.0, 100.0);
                    let clipped = c.draw_state.scissor([250, 250, 150, 150]);
                    Image::new().draw(&rust_logo, &clipped, transform, g);

                    let transform = c.transform.trans(200.0, 200.0);
                    Ellipse::new([1.0, 0.0, 0.0, 1.0]).draw(
                        [0.0, 0.0, 50.0, 50.0],
                        &DrawState::new_clip(),
                        transform,
                        g,
                    );
                    Image::new().draw(
                        &rust_logo,
                        &if clip_inside {
                            DrawState::new_inside()
                        } else {
                            DrawState::new_outside()
                        },
                        transform,
                        g,
                    );
                },
            );
            queue.submit(std::iter::once(command_buffer));
            surface_texture.present();
        });

        if let Some(Button::Keyboard(Key::A)) = event.press_args() {
            blend = (blend + 1) % blends.len();
            println!("Changed blending to {:?}", blends[blend]);
        }
        if let Some(Button::Keyboard(Key::S)) = event.press_args() {
            clip_inside = !clip_inside;
            if clip_inside {
                println!("Changed to clip inside");
            } else {
                println!("Changed to clip outside");
            }
        }
    }
}
