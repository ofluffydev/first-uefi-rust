#![no_main]
#![no_std]

pub mod tui;

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use embedded_graphics::prelude::RgbColor;
use libm::roundf;
use log::info;
use tui::tui_menu;
use uefi::proto::console::gop::{BltOp, BltRegion};
use uefi::{
    Result,
    boot::open_protocol_exclusive,
    prelude::*,
    proto::{
        console::{
            gop::{BltPixel, GraphicsOutput},
            text::Output,
        },
        rng::Rng,
    },
};
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::Pixel;

#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

impl Point {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

struct Buffer {
    width: usize,
    height: usize,
    pixels: Vec<BltPixel>,
}

impl Buffer {
    fn new(width: usize, height: usize) -> Self {
        Buffer {
            width,
            height,
            pixels: vec![BltPixel::new(0, 0, 0); width * height],
        }
    }

    fn pixel(&mut self, x: usize, y: usize) -> Option<&mut BltPixel> {
        self.pixels.get_mut(y * self.width + x)
    }

    fn blit(&self, gop: &mut GraphicsOutput) -> Result {
        gop.blt(BltOp::BufferToVideo {
            buffer: &self.pixels,
            src: BltRegion::Full,
            dest: (0, 0),
            dims: (self.width, self.height),
        })
    }

    fn blit_pixel(&self, gop: &mut GraphicsOutput, coords: (usize, usize)) -> Result {
        gop.blt(BltOp::BufferToVideo {
            buffer: &self.pixels,
            src: BltRegion::SubRectangle {
                coords,
                px_stride: self.width,
            },
            dest: coords,
            dims: (1, 1),
        })
    }
}

impl DrawTarget for Buffer {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> core::result::Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels {
            let (x, y) = TryInto::<(i32, i32)>::try_into(coord).expect("Invalid coordinate");
            if let Some(pixel) = self.pixel(x as usize, y as usize) {
                pixel.red = color.r();
                pixel.green = color.g();
                pixel.blue = color.b();
            }
        }
        Ok(())
    }
}

impl OriginDimensions for Buffer {
    fn size(&self) -> embedded_graphics::geometry::Size {
        embedded_graphics::geometry::Size::new(self.width as u32, self.height as u32)
    }
}

fn get_random_usize(rng: &mut Rng) -> usize {
    let mut buf = [0; size_of::<usize>()];
    rng.get_rng(None, &mut buf).expect("get_rng failed");
    usize::from_le_bytes(buf)
}

fn draw_sierpinski() -> Result {
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)?;

    let rng_handle = boot::get_handle_for_protocol::<Rng>()?;
    let mut rng = boot::open_protocol_exclusive::<Rng>(rng_handle)?;

    let (width, height) = gop.current_mode_info().resolution();
    let mut buffer = Buffer::new(width, height);

    for y in 0..height {
        let r = ((y as f32) / ((height - 1) as f32)) * 255.0;
        for x in 0..width {
            let g = ((x as f32) / ((width - 1) as f32)) * 255.0;
            let pixel = buffer.pixel(x, y).unwrap();
            pixel.red = r as u8;
            pixel.green = g as u8;
            pixel.blue = 255;
        }
    }

    buffer.blit(&mut gop)?;

    let size = Point::new(width as f32, height as f32);

    let border = 20.0;
    let triangle = [
        Point::new(size.x / 2.0, border),
        Point::new(border, size.y - border),
        Point::new(size.x - border, size.y - border),
    ];

    let mut p = Point::new(size.x / 2.0, size.y / 2.0);

    loop {
        let v = triangle[get_random_usize(&mut rng) % 3];

        p.x = (p.x + v.x) * 0.5;
        p.y = (p.y + v.y) * 0.5;

        let pixel = buffer.pixel(p.x as usize, p.y as usize).unwrap();
        pixel.red = 0;
        pixel.green = 100;
        pixel.blue = 0;

        buffer.blit_pixel(&mut gop, (p.x as usize, p.y as usize))?;
    }
}

fn load_image(image_data: &[u8], width: usize, height: usize) -> Result {
    let gop_handle = boot::get_handle_for_protocol::<GraphicsOutput>()?;
    let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(gop_handle)?;

    let (screen_width, screen_height) = gop.current_mode_info().resolution();

    let scale_x = screen_width as f32 / width as f32;
    let scale_y = screen_height as f32 / height as f32;
    let scale = scale_x.min(scale_y);

    let offset_x = roundf((screen_width as f32 - (width as f32 * scale)) / 2.0) as usize;
    let offset_y = roundf((screen_height as f32 - (height as f32 * scale)) / 2.0) as usize;

    let mut buffer = Buffer::new(screen_width, screen_height);

    for y in 0..height {
        for x in 0..width {
            let pixel_index = (y * width + x) * 3;
            let red = image_data[pixel_index];
            let green = image_data[pixel_index + 1];
            let blue = image_data[pixel_index + 2];

            let scaled_x = roundf(x as f32 * scale) as usize + offset_x;
            let scaled_y = roundf(y as f32 * scale) as usize + offset_y;

            if let Some(pixel) = buffer.pixel(scaled_x, scaled_y) {
                pixel.red = red;
                pixel.green = green;
                pixel.blue = blue;
            }
        }
    }

    buffer.blit(&mut gop)?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum BootChoice {
    Option1,
    Option2,
    Option3,
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();

    let handle = uefi::boot::get_handle_for_protocol::<Output>().unwrap();
    let mut output = open_protocol_exclusive::<Output>(handle).unwrap();
    output.clear().expect("Failed to clear screen");

    info!("boyloader online!");
    info!("Hai Ary! I love you :3");

    info!("Loading TUI...");

    let choice = tui_menu();

    output.clear().expect("Failed to clear screen");

    match choice {
        BootChoice::Option1 => info!("Option1 selected!"),
        BootChoice::Option2 => info!("Option2 selected!"),
        BootChoice::Option3 => info!("Option3 selected!"),
    }

    boot::stall(10_000_000);
    Status::SUCCESS
}
