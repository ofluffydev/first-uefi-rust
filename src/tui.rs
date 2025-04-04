use crate::{BootChoice, Buffer};
use alloc::{format, vec::Vec};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::Rgb888,
    prelude::*,
    text::{Alignment, Text},
};
use log::error;
use uefi::Result;
use uefi::{
    Status,
    proto::console::text::{Key, ScanCode},
};
use uefi::{
    boot::{get_handle_for_protocol, open_protocol_exclusive},
    proto::console::{gop::GraphicsOutput, text::Input},
};

pub fn tui_menu() -> BootChoice {
    let choice = render_loop().unwrap();
    return choice;
}

fn render_loop() -> Result<BootChoice> {
    let gop_handle = get_handle_for_protocol::<GraphicsOutput>()?;
    let mut gop = open_protocol_exclusive::<GraphicsOutput>(gop_handle)?;

    let (width, height) = gop.current_mode_info().resolution();

    let mut buffer = Buffer::new(width, height);

    let character_style = MonoTextStyle::new(&FONT_6X10, Rgb888::new(255, 255, 255));

    let stdin_handle = get_handle_for_protocol::<Input>()?;
    let mut stdin = open_protocol_exclusive::<Input>(stdin_handle)?;

    let black = Rgb888::new(0, 0, 0);

    let ppm_data = include_bytes!("../art/boykisser.ppm");

    let menu_items = [
        BootChoice::Option1,
        BootChoice::Option2,
        BootChoice::Option3,
    ];
    let mut selected_index = 0;

    loop {
        buffer.clear(black);
        for (i, item) in menu_items.iter().enumerate() {
            let prefix = if i == selected_index { ">> " } else { "   " };
            let text_str = format!("{}{:?}", prefix, item);
            Text::new(
                &text_str,
                Point::new(50, 150 + (i as i32 * 20)),
                character_style,
            )
            .draw(&mut buffer)
            .unwrap();
        }
        buffer.blit(&mut gop)?;
        match stdin.read_key() {
            Ok(Some(Key::Special(ScanCode::UP))) => {
                if selected_index > 0 {
                    selected_index -= 1;
                }
            }
            Ok(Some(Key::Special(ScanCode::DOWN))) => {
                if selected_index < menu_items.len() - 1 {
                    selected_index += 1;
                }
            }
            Ok(Some(Key::Printable(key))) if key == '\r' => {
                buffer.clear(black);
                let (image_width, image_height, image_pixels) = parse_ppm(ppm_data)?;
                let x_offset = (width as i32 - image_width as i32) / 2;
                let y_offset = (height as i32 - image_height as i32) / 2;
                draw_image(
                    &mut buffer,
                    image_pixels,
                    black,
                    width,
                    height,
                    x_offset,
                    y_offset,
                    image_width,
                    image_height,
                );
                let selected_text = format!("You selected: {:?}", menu_items[selected_index]);
                Text::with_alignment(
                    &selected_text,
                    Point::new(width as i32 / 2, (height as i32 / 2) + (0.3 * height as f32) as i32),
                    MonoTextStyle::new(&FONT_6X10, Rgb888::new(120, 0, 255)),
                    Alignment::Center,
                )
                .draw(&mut buffer)
                .expect("Failed to draw selected choice");
                buffer.blit(&mut gop)?;
                return Ok(menu_items[selected_index]);
            }
            Ok(Some(Key::Special(ScanCode::ESCAPE))) => {
                // Return a default choice (e.g., Option1) when ESC is pressed.
                return Ok(BootChoice::Option1);
            }
            Err(e) => {
                buffer.clear(black);
                error!("Error reading key: {:?}", e);
                let error_message = format!("Error: {:?}", e);
                Text::with_alignment(
                    &error_message,
                    Point::new(width as i32 / 2, height as i32 / 2),
                    character_style,
                    Alignment::Center,
                )
                .draw(&mut buffer)
                .expect("Failed to draw error message");
                buffer.blit(&mut gop).expect("Failed to blit error message");
            }
            _ => {}
        }
    }
}

fn draw_image(
    buffer: &mut Buffer,
    image_pixels: &[u8],
    black: Rgb888,
    width: usize,
    height: usize,
    x_offset: i32,
    y_offset: i32,
    image_width: usize,
    image_height: usize,
) {
    buffer.clear(black);

    for y in 0..image_height {
        for x in 0..image_width {
            let pixel_index = (y * image_width + x) * 3;
            if pixel_index + 2 >= image_pixels.len() {
                error!("Pixel index out of bounds: {}", pixel_index);
                break;
            }

            let red = image_pixels[pixel_index];
            let green = image_pixels[pixel_index + 1];
            let blue = image_pixels[pixel_index + 2];

            let screen_x = x as i32 + x_offset;
            let screen_y = y as i32 + y_offset;

            if screen_x >= 0
                && screen_x < width as i32
                && screen_y >= 0
                && screen_y < height as i32
            {
                if let Some(pixel) = buffer.pixel(screen_x as usize, screen_y as usize) {
                    pixel.red = red;
                    pixel.green = green;
                    pixel.blue = blue;
                }
            }
        }
    }
}

fn parse_ppm(data: &[u8]) -> Result<(usize, usize, &[u8])> {
    // Parse a simple PPM (P6) file.
    let mut lines = data.split(|&b| b == b'\n');
    let header = lines.next().ok_or(Status::INVALID_PARAMETER)?;
    if header != b"P6" {
        return Err(Status::INVALID_PARAMETER.into());
    }

    let dimensions = lines
        .next()
        .ok_or(Status::INVALID_PARAMETER)?
        .split(|&b| b == b' ')
        .collect::<Vec<_>>();
    if dimensions.len() != 2 {
        return Err(Status::INVALID_PARAMETER.into());
    }

    let width = core::str::from_utf8(dimensions[0])
        .map_err(|_| Status::INVALID_PARAMETER)?
        .parse::<usize>()
        .map_err(|_| Status::INVALID_PARAMETER)?;
    let height = core::str::from_utf8(dimensions[1])
        .map_err(|_| Status::INVALID_PARAMETER)?
        .parse::<usize>()
        .map_err(|_| Status::INVALID_PARAMETER)?;

    let max_val = core::str::from_utf8(lines.next().ok_or(Status::INVALID_PARAMETER)?)
        .map_err(|_| Status::INVALID_PARAMETER)?
        .parse::<usize>()
        .map_err(|_| Status::INVALID_PARAMETER)?;
    if max_val != 255 {
        return Err(Status::INVALID_PARAMETER.into());
    }

    let pixel_data_start = data.len() - (width * height * 3);
    Ok((width, height, &data[pixel_data_start..]))
}
