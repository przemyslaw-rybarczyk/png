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

pub fn unfilter_uninterlace(data: &mut [u8], pixels: &mut [u8], pitch: usize, width: u32, height: u32, color_mode: &ColorMode, interlace_method: InterlaceMethod) -> Result<()> {
    let width = width as usize;
    let height = height as usize;
    match interlace_method {
        InterlaceMethod::NoInterlace => {
            let bytes_per_scanline = (width * color_mode.bits_per_pixel() + 7) / 8 + 1;
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
                match color_mode {
                    ColorMode::RGBA8 => {
                        for x in 0 .. width {
                            let i = y * bytes_per_scanline + 1 + x * 4;
                            let j = y * pitch + x * 4;
                            pixels[j + 0] = data[i + 2];
                            pixels[j + 1] = data[i + 1];
                            pixels[j + 2] = data[i + 0];
                            pixels[j + 3] = data[i + 3];
                        }
                    },
                    _ => std::todo!(),
                }
            }
        },
        InterlaceMethod::Adam7 => std::todo!(),
    }
    Ok(())
}
