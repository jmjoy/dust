use std::error::Error;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::surface::Surface;
use sdl2::video::WindowContext;
use sdl2::{Sdl, VideoSubsystem};


pub const SCREEN_WIDTH: u32 = 256;
pub const SCREEN_HEIGHT: u32 = 192 * 2;

pub struct Renderer<'a> {
    texture: Texture<'a>, // TODO - what happens if _tc is destroyed first ?
    canvas: WindowCanvas,
    #[allow(unused)]
    tc: TextureCreator<WindowContext>, // only kept alive because of the texture
    #[allow(unused)]
    video_subsystem: VideoSubsystem, // holds a reference to the video subsystem
}

pub fn init(sdl_context: &Sdl) -> anyhow::Result<Renderer> {
    let video_subsystem = sdl_context.video().map_err(anyhow::Error::msg)?;
    let mut window = video_subsystem
        .window("dust-sdl2", SCREEN_WIDTH, SCREEN_HEIGHT)
        .opengl()
        .position_centered()
        .resizable()
        .build()?;

    let mut canvas = window.into_canvas().accelerated().build()?;
    canvas.set_logical_size(SCREEN_WIDTH, SCREEN_HEIGHT)?;

    let mut tc = canvas.texture_creator();
    let texture = unsafe {
        let tc_ptr = &mut tc as *mut TextureCreator<WindowContext>;
        (*tc_ptr)
            .create_texture_streaming(PixelFormatEnum::BGRA32, SCREEN_WIDTH, SCREEN_HEIGHT)
            .unwrap()
    };

    Ok(Renderer {
        tc,
        texture,
        canvas,
        video_subsystem,
    })
}

impl<'a> Renderer<'a> {
    pub fn set_window_title(&mut self, title: &str) {
        self.canvas.window_mut().set_title(&title).unwrap();
    }

    pub fn render(&mut self, buffer: &[u32]) {
        self.texture
            .update(
                None,
                unsafe { std::mem::transmute::<&[u32], &[u8]>(buffer) },
                (SCREEN_WIDTH as usize) * 4,
            )
            .unwrap();
        self.canvas.set_draw_color(Color::RGB(0, 0, 0));
        self.canvas.clear();
        self.canvas
            .copy(
                &self.texture,
                None,
                Some(Rect::new(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT)),
            )
            .unwrap();
        self.canvas.present();
    }
}
