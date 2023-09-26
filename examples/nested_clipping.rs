mod include;

use crate::include::{event_resize, init_surface_config};
use graphics::{
    clear,
    draw_state::{Blend, DrawState, Stencil},
    Rectangle,
};
use piston::{EventSettings, Events, PressEvent, RenderEvent, WindowSettings};
use winit_window::WinitWindow;

fn main() {
    let mut window = WinitWindow::new(&WindowSettings::new(
        "wgpu_graphics: nested_clipping",
        (640, 480),
    ));

    let instance = wgpu::Instance::new(Default::default());
    let surface = unsafe { instance.create_surface(window.get_window()) }.unwrap();
    let adapter =
        futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .unwrap();

    let mut device_descriptor = wgpu::DeviceDescriptor::default();
    device_descriptor.features.set(wgpu::Features::DEPTH_CLIP_CONTROL, true);
    let (device, queue) = futures::executor::block_on(
        adapter.request_device(&device_descriptor, None),
    )
    .unwrap();
    let mut surface_config = init_surface_config(&surface, &adapter, &window);

    surface.configure(&device, &surface_config);

    let mut wgpu2d = wgpu_graphics::Wgpu2d::new(&device, &surface_config);
    let mut events = Events::new(EventSettings::new());

    let increment = DrawState::new_increment();
    let inside_level1 = DrawState {
        blend: Some(Blend::Alpha),
        stencil: Some(Stencil::Inside(1)),
        scissor: None,
    };
    let inside_level2 = DrawState {
        blend: Some(Blend::Alpha),
        stencil: Some(Stencil::Inside(2)),
        scissor: None,
    };
    let inside_level3 = DrawState {
        blend: Some(Blend::Alpha),
        stencil: Some(Stencil::Inside(3)),
        scissor: None,
    };
    let mut clip = true;
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
                    clear([0.8, 0.8, 0.8, 1.0], g);

                    if clip {
                        Rectangle::new([1.0; 4]).draw(
                            [10.0, 10.0, 200.0, 200.0],
                            &increment,
                            c.transform,
                            g,
                        );
                        Rectangle::new([1.0, 0.0, 0.0, 1.0]).draw(
                            [10.0, 10.0, 200.0, 200.0],
                            &inside_level1,
                            c.transform,
                            g,
                        );

                        Rectangle::new([1.0; 4]).draw(
                            [100.0, 100.0, 200.0, 200.0],
                            &increment,
                            c.transform,
                            g,
                        );
                        Rectangle::new([0.0, 0.0, 1.0, 1.0]).draw(
                            [100.0, 100.0, 200.0, 200.0],
                            &inside_level2,
                            c.transform,
                            g,
                        );

                        Rectangle::new([1.0; 4]).draw(
                            [100.0, 100.0, 200.0, 200.0],
                            &increment,
                            c.transform,
                            g,
                        );
                        Rectangle::new([0.0, 1.0, 0.0, 1.0]).draw(
                            [50.0, 50.0, 200.0, 100.0],
                            &inside_level3,
                            c.transform,
                            g,
                        );
                    } else {
                        Rectangle::new([1.0, 0.0, 0.0, 1.0]).draw(
                            [10.0, 10.0, 200.0, 200.0],
                            &c.draw_state,
                            c.transform,
                            g,
                        );

                        Rectangle::new([0.0, 0.0, 1.0, 1.0]).draw(
                            [100.0, 100.0, 200.0, 200.0],
                            &c.draw_state,
                            c.transform,
                            g,
                        );

                        Rectangle::new([0.0, 1.0, 0.0, 1.0]).draw(
                            [50.0, 50.0, 200.0, 100.0],
                            &c.draw_state,
                            c.transform,
                            g,
                        );
                    }
                },
            );
            queue.submit(std::iter::once(command_buffer));
            surface_texture.present();
        });
        event.press(|_| {
            clip = !clip;
        });
    }
}
