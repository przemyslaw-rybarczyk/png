use crate::ihdr::ColorMode;
use crate::ihdr::InterlaceMethod;
use crate::Error;
use crate::Result;

#[derive(Copy, Clone)]
enum FilterType {
    None,
    Sub,
    Up,
    Average,
    Paeth,
}

impl FilterType {
    fn read(filter_type: u8) -> Result<FilterType> {
        use FilterType::*;
        match filter_type {
            0 => Ok(None),
            1 => Ok(Sub),
            2 => Ok(Up),
            3 => Ok(Average),
            4 => Ok(Paeth),
            _ => Err(Error::Format("Invalid filter type")),
        }
    }
}

fn color_16_to_8(upper: u8, lower: u8) -> u8 {
    ((upper as f64 * 256.0 + lower as f64) * 255.0 / 65535.0).round() as u8
}

pub fn unfilter_uninterlace(data: &mut [u8], pixels: &mut [u8], pitch: usize, width: u32, height: u32, color_mode: &ColorMode, interlace_method: InterlaceMethod) -> Result<()> {
    let width = width as usize;
    let height = height as usize;
    match interlace_method {
        InterlaceMethod::NoInterlace => {
            let bits_per_pixel = color_mode.bits_per_pixel();
            let bytes_per_scanline = (width * bits_per_pixel + 7) / 8 + 1;
            let filter_bpp = (color_mode.bits_per_pixel() + 7) / 8;
            for y in 0 .. height {
                let filter_type = FilterType::read(data[y * bytes_per_scanline])?;
                for x in 0 .. bytes_per_scanline - 1 {
                    let a = if x >= filter_bpp { data[y * bytes_per_scanline + 1 + x - filter_bpp] as i16 } else { 0 };
                    let b = if y > 0 { data[(y - 1) * bytes_per_scanline + 1 + x] as i16 } else { 0 };
                    let c = if x >= filter_bpp && y > 0 { data[(y - 1) * bytes_per_scanline + 1 + x - filter_bpp] as i16 } else { 0 };
                    data[y * bytes_per_scanline + 1 + x] = u8::wrapping_add(data[y * bytes_per_scanline + 1 + x], match filter_type {
                        FilterType::None => 0,
                        FilterType::Sub => a,
                        FilterType::Up => b,
                        FilterType::Average => (a + b) / 2,
                        FilterType::Paeth => {
                            let p = a + b - c;
                            let pa = i16::abs(p - a);
                            let pb = i16::abs(p - b);
                            let pc = i16::abs(p - c);
                            if pa <= pb && pa <= pc {a} else if pb <= pc {b} else {c}
                        },
                    } as u8);
                }
            }
            for y in 0 .. height {
                for x in 0 .. width {
                    let i = y * bytes_per_scanline + 1 + x * bits_per_pixel / 8;
                    let j = y * pitch + x * 4;
                    let color = match color_mode {
                        ColorMode::Grayscale1 => {
                            let shift = (7 - x % 8) as u32;
                            let color = (data[i].wrapping_shr(shift) & 0x01) * (255 / 1);
                            (color, color, color, 255)
                        },
                        ColorMode::Grayscale2 => {
                            let shift = ((3 - x % 4) * 2) as u32;
                            let color = (data[i].wrapping_shr(shift) & 0x03) * (255 / 3);
                            (color, color, color, 255)
                        },
                        ColorMode::Grayscale4 => {
                            let shift = ((1 - x % 2) * 4) as u32;
                            let color = (data[i].wrapping_shr(shift) & 0x0F) * (255 / 15);
                            (color, color, color, 255)
                        },
                        ColorMode::Grayscale8 => (data[i], data[i], data[i], 255),
                        ColorMode::Grayscale16 => {
                            let color = color_16_to_8(data[i + 0], data[i + 1]);
                            (color, color, color, 255)
                        },
                        ColorMode::RGB8 => (data[i + 0], data[i + 1], data[i + 2], 255),
                        ColorMode::RGB16 => (
                            color_16_to_8(data[i + 0], data[i + 1]),
                            color_16_to_8(data[i + 2], data[i + 3]),
                            color_16_to_8(data[i + 4], data[i + 5]),
                            255,
                        ),
                        ColorMode::GrayscaleAlpha8 => (data[i + 0], data[i + 0], data[i + 0], data[i + 1]),
                        ColorMode::GrayscaleAlpha16 => {
                            let color = color_16_to_8(data[i + 0], data[i + 1]);
                            let alpha = color_16_to_8(data[i + 2], data[i + 3]);
                            (color, color, color, alpha)
                        },
                        ColorMode::RGBA8 => (data[i + 0], data[i + 1], data[i + 2], data[i + 3]),
                        ColorMode::RGBA16 => (
                            color_16_to_8(data[i + 0], data[i + 1]),
                            color_16_to_8(data[i + 2], data[i + 3]),
                            color_16_to_8(data[i + 4], data[i + 5]),
                            color_16_to_8(data[i + 6], data[i + 7]),
                        ),
                        _ => std::todo!(),
                    };
                    pixels[j + 0] = color.2;
                    pixels[j + 1] = color.1;
                    pixels[j + 2] = color.0;
                    pixels[j + 3] = color.3;
                }
            }
        },
        InterlaceMethod::Adam7 => std::todo!(),
    }
    Ok(())
}
