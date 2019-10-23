extern crate num;
extern crate rayon;
extern crate sdl2;

use num::{Complex, One, Zero};
use rayon::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::time::Duration;

/// The mapping we want to apply is defined here. Return
/// Some(value) if the mapping is successful, return None if it's
/// not defined in the given region.
fn conformal_mapping(c: Complex<f64>) -> Option<Complex<f64>> {
    if c != Complex::zero() {
        Some(Complex::<f64>::one() / c)
    } else {
        None
    }
}

/// Use multithreading to map all the points in the slice.
fn remap(grid: &[Complex<f64>]) -> Vec<Option<Complex<f64>>> {
    grid.par_iter().map(|c| conformal_mapping(*c)).collect()
}

/// Convert a complex value to a pixel coordinate on the screen.
fn complex_to_point(
    c: &Option<Complex<f64>>,
    scale: f64,
    pixel_side: u32,
) -> Option<sdl2::rect::Point> {
    if let Some(c) = c {
        let pixel_scale = f64::from(pixel_side) / (2.0 * scale);
        let x = ((c.re + scale) * pixel_scale) as i32;
        let y = ((c.im + scale) * pixel_scale) as i32;
        Some(sdl2::rect::Point::new(x, y))
    } else {
        None
    }
}

/// Draw the line if both points were computed successfully.
fn draw_if_both(
    a: &Option<Point>,
    b: &Option<Point>,
    canvas: &mut Canvas<Window>,
) -> Result<(), String> {
    if let Some(a) = a {
        if let Some(b) = b {
            canvas.draw_line(*a, *b)?;
        }
    }
    Ok(())
}

/// A type which stores possible connections to neighboring indices.
type Connection = (
    usize,
    Option<usize>,
    Option<usize>,
    Option<usize>,
    Option<usize>,
);

/// Draw the whole mapped grid, and lines.
fn draw_mapped_grid(
    grid_connections: &[Connection],
    mapped_grid: &[Option<Complex<f64>>],
    mapped_lines: &[&[Option<Complex<f64>>]],
    scale: f64,
    window_size: u32,
    canvas: &mut Canvas<Window>,
) -> Result<(), String> {
    grid_connections
        .iter()
        .try_for_each(|(pos, left, right, up, down)| -> Result<(), String> {
            let pos_c = mapped_grid[*pos];
            let point_c = complex_to_point(&pos_c, scale, window_size);

            let left =
                left.and_then(|left| complex_to_point(&mapped_grid[left], scale, window_size));
            let right =
                right.and_then(|right| complex_to_point(&mapped_grid[right], scale, window_size));
            let up = up.and_then(|up| complex_to_point(&mapped_grid[up], scale, window_size));
            let down =
                down.and_then(|down| complex_to_point(&mapped_grid[down], scale, window_size));

            draw_if_both(&point_c, &left, canvas)?;
            draw_if_both(&point_c, &right, canvas)?;
            draw_if_both(&point_c, &up, canvas)?;
            draw_if_both(&point_c, &down, canvas)?;
            Ok(())
        })
        .unwrap();

    canvas.set_draw_color(Color::RGB(0, 255, 0));
    mapped_lines.iter().for_each(|line| {
        line.windows(2)
            .try_for_each(|cs| -> Result<(), String> {
                let pos_a = cs[0];
                let point_a = complex_to_point(&pos_a, scale, window_size);
                let pos_b = cs[1];
                let point_b = complex_to_point(&pos_b, scale, window_size);
                draw_if_both(&point_a, &point_b, canvas)?;
                Ok(())
            })
            .unwrap();
    });

    canvas.set_draw_color(Color::RGB(255, 255, 255));

    let a = Complex { re: -1.0, im: -1.0 };
    let b = Complex { re: 1.0, im: 1.0 };
    let point_a = complex_to_point(&Some(a), scale, window_size);
    let point_b = complex_to_point(&Some(b), scale, window_size);
    let d = point_b.and_then(|b| point_a.map(|a| b - a));
    if let Some(d) = d {
        let a = point_a.unwrap();
        canvas.draw_rect(Rect::new(a.x(), a.y(), d.x() as u32, d.y() as u32))
    } else {
        Ok(())
    }
}

/// Main function to set up the window and start the loop.
pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window_size = 800;
    let window = video_subsystem
        .window("Conformal Mappings", window_size, window_size)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut scale = 1.0; // Scale for points
    let grid_length: usize = 101;
    let l = (grid_length / 2) as i64;
    let gridpoints: Vec<Complex<f64>> = (-l..grid_length as i64 - l)
        .map(|re| -> Vec<Complex<f64>> {
            (-l..grid_length as i64 - l)
                .map(|im| -> Complex<f64> {
                    Complex {
                        re: re as f64 / l as f64,
                        im: im as f64 / l as f64,
                    }
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();

    let xaxis = (-l..grid_length as i64 - l)
        .map(|re| -> Complex<f64> {
            Complex {
                re: re as f64 / l as f64,
                im: 0.0,
            }
        })
        .collect::<Vec<_>>();
    let yaxis = (-l..grid_length as i64 - l)
        .map(|im| -> Complex<f64> {
            Complex {
                re: 0.0,
                im: im as f64 / l as f64,
            }
        })
        .collect::<Vec<_>>();
    let unit_circle = (-l..grid_length as i64 - l)
        .map(|im| -> Complex<f64> {
            Complex {
                re: 0.0,
                im: 2.0 * std::f64::consts::PI * im as f64 / l as f64,
            }
            .exp()
        })
        .collect::<Vec<_>>();

    let mut mapped_points = remap(&gridpoints);
    let mut mapped_x = remap(&xaxis);
    let mut mapped_y = remap(&yaxis);
    let mut mapped_unit_circle = remap(&unit_circle);

    let map_to_index = |x: usize, y: usize| -> usize { x + (y * grid_length) };
    let connections: Vec<Connection> = (0..gridpoints.len())
        .map(|pos| -> Connection {
            let x = pos % grid_length;
            let y = pos / grid_length;

            let left = if x > 0 {
                Some(map_to_index(x - 1, y))
            } else {
                None
            };
            let right = if x < grid_length - 1 {
                Some(map_to_index(x + 1, y))
            } else {
                None
            };
            let up = if y > 0 {
                Some(map_to_index(x, y - 1))
            } else {
                None
            };
            let down = if y < grid_length - 1 {
                Some(map_to_index(x, y + 1))
            } else {
                None
            };
            (pos, left, right, up, down)
        })
        .collect();

    'running: loop {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running Ok(()),
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => {
                    if scale > 0.1 {
                        scale *= 0.9;
                    }
                    mapped_points = remap(&gridpoints);
                    mapped_x = remap(&xaxis);
                    mapped_y = remap(&yaxis);
                    mapped_unit_circle = remap(&unit_circle);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => {
                    if scale < 10.0 {
                        scale *= 1.0 / 0.9;
                    }
                    mapped_points = remap(&gridpoints);
                    mapped_x = remap(&xaxis);
                    mapped_unit_circle = remap(&unit_circle);
                }
                _ => {}
            }
        }
        canvas.set_draw_color(Color::RGB(255, 0, 0));

        draw_mapped_grid(
            &connections,
            &mapped_points,
            &[&mapped_x, &mapped_y, &mapped_unit_circle],
            scale,
            window_size,
            &mut canvas,
        )?;

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
