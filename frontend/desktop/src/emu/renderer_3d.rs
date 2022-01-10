use dust_core::{
    gpu::{
        engine_3d::{Polygon, Renderer as RendererTrair, Vertex},
        Scanline, SCREEN_HEIGHT, SCREEN_WIDTH,
    },
    utils::Bytes,
};
use std::{
    cell::UnsafeCell,
    hint,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    thread,
};

struct RenderingData {
    texture: Bytes<0x8_0000>,
    tex_pal: Bytes<0x1_8000>,
    vert_ram: [Vertex; 6144],
    poly_ram: [Polygon; 2048],
    vert_ram_level: u16,
    poly_ram_level: u16,
}

struct SharedData {
    rendering_data: UnsafeCell<RenderingData>,
    scanline_buffer: UnsafeCell<[Scanline<u32, SCREEN_WIDTH>; SCREEN_HEIGHT]>,
    processing_scanline: AtomicU8,
    stopped: AtomicBool,
}

unsafe impl Sync for SharedData {}

pub struct Renderer {
    next_scanline: u8,
    shared_data: Arc<SharedData>,
    thread: Option<thread::JoinHandle<()>>,
}

impl RendererTrair for Renderer {
    fn swap_buffers(
        &mut self,
        texture: &Bytes<0x8_0000>,
        tex_pal: &Bytes<0x1_8000>,
        vert_ram: &[Vertex],
        poly_ram: &[Polygon],
        state: &dust_core::gpu::engine_3d::RenderingState,
    ) {
        let rendering_data = unsafe { &mut *self.shared_data.rendering_data.get() };

        for i in 0..4 {
            if state.texture_dirty & 1 << i == 0 {
                continue;
            }
            let range = i << 17..(i + 1) << 17;
            rendering_data.texture[range.clone()].copy_from_slice(&texture[range]);
        }
        for i in 0..6 {
            if state.tex_pal_dirty & 1 << i == 0 {
                continue;
            }
            let range = i << 14..(i + 1) << 14;
            rendering_data.tex_pal[range.clone()].copy_from_slice(&tex_pal[range]);
        }
        rendering_data.vert_ram[..vert_ram.len()].copy_from_slice(vert_ram);
        rendering_data.poly_ram[..poly_ram.len()].copy_from_slice(poly_ram);
        rendering_data.vert_ram_level = vert_ram.len() as u16;
        rendering_data.poly_ram_level = poly_ram.len() as u16;

        self.shared_data
            .processing_scanline
            .store(u8::MAX, Ordering::Release);
        self.thread.as_ref().unwrap().thread().unpark();
    }

    fn start_frame(&mut self) {
        self.next_scanline = 0;
    }

    fn read_scanline(&mut self) -> &Scanline<u32, SCREEN_WIDTH> {
        while {
            let processing_scanline = self.shared_data.processing_scanline.load(Ordering::Acquire);
            processing_scanline == u8::MAX || processing_scanline <= self.next_scanline
        } {
            hint::spin_loop();
        }
        let result =
            unsafe { &(&*self.shared_data.scanline_buffer.get())[self.next_scanline as usize] };
        self.next_scanline += 1;
        result
    }

    fn skip_scanline(&mut self) {
        self.next_scanline += 1;
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.shared_data.stopped.store(true, Ordering::Relaxed);
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

impl Renderer {
    pub fn new() -> Self {
        let shared_data = Arc::new(SharedData {
            rendering_data: UnsafeCell::new(RenderingData {
                texture: Bytes::new([0; 0x8_0000]),
                tex_pal: Bytes::new([0; 0x1_8000]),
                vert_ram: [Vertex::new(); 6144],
                poly_ram: [Polygon::new(); 2048],
                vert_ram_level: 0,
                poly_ram_level: 0,
            }),
            scanline_buffer: UnsafeCell::new([Scanline([0; SCREEN_WIDTH]); SCREEN_HEIGHT]),
            processing_scanline: AtomicU8::new(SCREEN_HEIGHT as u8),
            stopped: AtomicBool::new(false),
        });
        Renderer {
            next_scanline: 0,
            shared_data: shared_data.clone(),
            thread: Some(
                thread::Builder::new()
                    .name("3D rendering".to_string())
                    .spawn(move || {
                        let mut state = RenderingState::new(shared_data);
                        loop {
                            loop {
                                if state.shared_data.stopped.load(Ordering::Relaxed) {
                                    return;
                                }
                                if state
                                    .shared_data
                                    .processing_scanline
                                    .compare_exchange(
                                        u8::MAX,
                                        0,
                                        Ordering::Acquire,
                                        Ordering::Acquire,
                                    )
                                    .is_ok()
                                {
                                    break;
                                } else {
                                    thread::park();
                                }
                            }
                            state.run_frame();
                        }
                    })
                    .expect("Couldn't spawn 3D rendering thread"),
            ),
        }
    }
}

struct RenderingState {
    shared_data: Arc<SharedData>,
}

impl RenderingState {
    fn new(shared_data: Arc<SharedData>) -> Self {
        RenderingState { shared_data }
    }

    fn run_frame(&mut self) {
        for i in 0..SCREEN_HEIGHT as u8 {
            if self
                .shared_data
                .processing_scanline
                .compare_exchange(i, i + 1, Ordering::Release, Ordering::Relaxed)
                .is_err()
            {
                return;
            }
            // TODO: Render
        }
    }
}