use graphics::{types::Color, DrawState, Graphics, ImageSize};

pub struct Texture {}

impl ImageSize for Texture {
    fn get_size(&self) -> (u32, u32) {
        todo!()
    }
}

pub struct WgpuGraphics {}

impl Graphics for WgpuGraphics {
    type Texture = Texture;

    fn clear_color(&mut self, color: Color) {
        todo!()
    }

    fn clear_stencil(&mut self, value: u8) {
        todo!()
    }

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
