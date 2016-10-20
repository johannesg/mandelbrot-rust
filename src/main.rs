extern crate sdl2;
extern crate num;
#[macro_use] extern crate chan;

use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::event::{Event};
use chan::{Sender, Receiver, WaitGroup};
use std::thread;
use std::io::Write;
use num::traits::Zero;
use num::complex::Complex64;

static WIDTH: u32 = 400;
static HEIGHT: u32 = 300;
static MAX_ITERATIONS: u32 = 20;
static NUM_THREADS: u32 = 8;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("Mandelbrot using rust + sdl2", WIDTH, HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();

    let mut renderer = window.renderer().build().unwrap();

    let buffer_len: usize = WIDTH as usize * HEIGHT as usize * 3 as usize;
    let mut buffer = vec!(0u8; buffer_len);

    let mut texture = renderer.create_texture_streaming(PixelFormatEnum::RGB24, WIDTH, HEIGHT).unwrap();
    let mut events = sdl_context.event_pump().unwrap();

    let top_left = Complex64::new(-2f64, 1f64);
    let bottom_right = Complex64::new(1f64, -1f64);

    let (work_tx, work_rx) = chan::async();
    let (result_tx, result_rx) = chan::async();
    let tick = chan::tick_ms(500);
    let wg = WaitGroup::new();

    for _ in 0..NUM_THREADS {
        wg.add(1);
        let wg = wg.clone();
        spawn_worker(work_rx.clone(), result_tx.clone(), wg, top_left, bottom_right);
    }

    generate_work(&work_tx);

    'event: loop {
        chan_select! {
            default => {},
            tick.recv() => {
                for event in events.poll_iter() {
                    match event {
                        Event::Quit { .. } => break 'event,
                        _ => continue
                    }
                }

                texture.with_lock(None, |mut screen_buffer: &mut [u8], pitch: usize| {
                    screen_buffer.write(&mut buffer).unwrap();
                }).unwrap();
                renderer.clear();
                renderer.copy(&texture, None, Some(Rect::new(0, 0, WIDTH, HEIGHT)));
                renderer.present();
            },
            result_rx.recv() -> message => {
                let (x, y, n) = message.unwrap();
                let c = ((n * 255) / MAX_ITERATIONS) as u8;
                let offset = (WIDTH * y * 3 + x * 3) as usize;
                buffer[offset] = c;
                buffer[offset + 1] = c;
                buffer[offset + 2] = c;
            }
        }
    }
}

fn generate_work(work_tx: &Sender<(u32, u32)>) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let _ = work_tx.send((x, y));
        }
    }
}

fn spawn_worker(work_rx: Receiver<(u32, u32)>, result_tx: Sender<(u32, u32, u32)>,
                wg: WaitGroup, top_left: Complex64, bottom_right: Complex64) {
    let delta = bottom_right - top_left;
    let scale = Complex64 {
        re: delta.re / WIDTH as f64,
        im: delta.im / HEIGHT as f64
    };

    thread::spawn(move || {
        for (x, y) in work_rx {
            let (xf, yf) = (x as f64, y as f64);
            let dot = Complex64::new(
                scale.re * xf + top_left.re,
                scale.im * yf + top_left.im
            );
            let n = iterate(dot);
            let _ = result_tx.send((x, y, n));
        }
        wg.done();
    });
}

fn iterate(c: Complex64) -> u32 {
    let mut z = Complex64::zero();
    let mut znorm = 0f64;
    let mut n = 0u32;

    while n < MAX_ITERATIONS && znorm < 4.0 {
        z = z * z + c;
        znorm = z.norm_sqr();
        n += 1;
    }
    n
}
