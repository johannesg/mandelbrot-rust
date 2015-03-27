#![feature(old_io)]
#![feature(std_misc)]

extern crate sdl2;
extern crate num;

use sdl2::video::{WindowPos, Window, OPENGL};
use sdl2::event::{Event};
use sdl2::surface::{Surface};
use std::sync::mpsc::{channel, Sender, Receiver};
#[allow(deprecated)]
use std::old_io::timer::Timer;
use std::time::Duration;
use std::thread;
use num::complex::Complex64;
use num::traits::Zero;

static WIDTH: i32 = 800;
static HEIGHT: i32 = 600;
static MAX_ITERATIONS: i32 = 100;
// static THREADS: i32 = 4;

fn main()
{
    // start sdl2
    let ctx = match sdl2::init(sdl2::INIT_VIDEO) {
        Ok(ctx)     => ctx,
        Err(err)    => panic!("Failed to start SDL2: {}", err)
    };

    // create a window
    let window = match Window::new("mandel", WindowPos::PosCentered, WindowPos::PosCentered, WIDTH, HEIGHT, OPENGL) {
        Ok(window)  => window,
        Err(err)    => panic!("Failed to create window: {}", err)
    };

    let screen = match window.get_surface() {
        Ok(screen)  => screen,
        Err(err)        => panic!("Failed to create screen: {}", err)
    };

    let buffer_len: usize = WIDTH as usize * HEIGHT as usize * 4 as usize;
    let mut buffer = vec!(0u8; buffer_len);

    let surface = match Surface::from_data(buffer.as_mut_slice(), WIDTH, HEIGHT, 32, WIDTH * 4, 0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000) {
        Ok(surface) => surface,
        Err(err)    => panic!("Failed to create surface: {}", err)
    };

    let mut timer = Timer::new().unwrap();
    let periodic = timer.periodic(Duration::milliseconds(50));

    let (work_tx, work_rx) = channel::<(i32,i32)>();
    let (result_tx, result_rx) = channel::<(i32,i32,i32)>();

    let top_left = Complex64::new(-2f64, 1f64);
    let bottom_right = Complex64::new(1f64, -1f64);

    // no support for multiple consumers???
    // for thread_id in 0..THREADS {
    spawn_worker(work_rx, result_tx, top_left, bottom_right);
    // }
    
    generate_work(work_tx);
    
    let mut events = ctx.event_pump();
    // loop until we receive a QuitEvent
    'event : loop {
        select! {
            res = result_rx.recv() => {
                if res.is_err() {
                    continue;
                }

                let (x, y, n) = res.unwrap();
                let c = ((n * 255) / MAX_ITERATIONS) as u8;
                let offset = (WIDTH * y * 4 + x * 4) as usize;
                buffer[offset] = c;
                buffer[offset+1] = c;
                buffer[offset+2] = c;
                buffer[offset+3] = 255;
            },
            _ = periodic.recv() => {
                // poll_event return the most recent event or NoEvent if nothing has happened
                for event in events.poll_iter() {
                    match event {
                        Event::Quit{..} => break 'event,
                        _               => continue
                    }
                }
                screen.blit(&surface, None, None);
                window.update_surface();
            }
        }
    }
}

fn generate_work(work: Sender<(i32,i32)>) {
    for y in 0..HEIGHT {
       for x in 0..WIDTH {
           let _ = work.send((x,y));
       } 
    }
}

fn spawn_worker(
    work: Receiver<(i32,i32)>, 
    result : Sender<(i32,i32,i32)>,
    top_left : Complex64,
    bottom_right : Complex64
    ) {

    let delta = bottom_right - top_left;

    let scale = Complex64 { 
        re: delta.re / WIDTH as f64, 
        im: delta.im / HEIGHT as f64
    };

    thread::spawn(move || {
        loop {
            let (x,y) = match work.recv() {
                Ok(res) => res,
                Err(_) => { println!("Complete"); break },
            };

            let (xf, yf) = (x as f64, y as f64);
            let dot = Complex64::new(
                scale.re * xf + top_left.re,
                scale.im * yf + top_left.im
                );
            let n = iterate(dot);

            let _ = result.send((x, y, n));
        }
    });
}

fn iterate(c : Complex64) -> i32 {
	let mut z = Complex64::zero();
	let mut znorm = 0f64;
	let mut n = 0i32;

	while n < MAX_ITERATIONS && znorm < 4.0 {
		z = z*z + c;
		znorm = z.norm_sqr();
        n += 1;
	}

    n
}

