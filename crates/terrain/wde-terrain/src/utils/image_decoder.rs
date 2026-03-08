use std::{fs::File, io::BufReader};

/// Utility function to decode a PNG image from the specified path, convert it to the target number of channels (1 for heightmap, 4 for splatmap), resize it to the target size if necessary, and return the raw pixel data as a byte vector. The function handles different PNG color types and applies the appropriate transformations to ensure the output data is in the expected format for terrain rendering.
/// 
/// # Arguments
/// * `path` - The file path to the PNG image to be decoded.
/// * `target_channels` - The desired number of channels in the output data (1 for heightmap, 4 for splatmap).
/// * `target_size` - The desired dimensions of the output image (width, height). If the source image has different dimensions, it will be resized using nearest neighbor scaling.
/// 
/// # Returns
/// A `Result` containing the raw pixel data as a byte vector if successful, or an error message as a string if the decoding or processing fails.
pub fn decode_png_as_channels(path: &str, target_channels: usize, target_size: (u32, u32)) -> Result<Vec<u8>, String> {
    let mut decoder = png::Decoder::new(BufReader::new(
        File::open(path).map_err(|e| format!("failed to open {path}: {e}"))?,
    ));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);

    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("failed to read PNG header for {path}: {e}"))?;
    let mut raw = vec![0; reader
        .output_buffer_size()
        .ok_or_else(|| format!("invalid output buffer size for {path}"))?];
    let info = reader
        .next_frame(&mut raw)
        .map_err(|e| format!("failed to decode PNG frame for {path}: {e}"))?;
    let raw = &raw[..info.buffer_size()];
    let src_size = (info.width, info.height);

    let channels_converted = match target_channels {
        1 => match info.color_type {
            png::ColorType::Grayscale => Ok(raw.to_vec()),
            png::ColorType::GrayscaleAlpha => Ok(raw.chunks_exact(2).map(|px| px[0]).collect()),
            png::ColorType::Rgb => Ok(raw.chunks_exact(3).map(|px| px[0]).collect()),
            png::ColorType::Rgba => Ok(raw.chunks_exact(4).map(|px| px[0]).collect()),
            other => Err(format!("unsupported PNG color type for heightmap {path}: {other:?}")),
        },
        4 => match info.color_type {
            png::ColorType::Rgba => Ok(raw.to_vec()),
            png::ColorType::Rgb => {
                let mut out = Vec::with_capacity((raw.len() / 3) * 4);
                for px in raw.chunks_exact(3) {
                    out.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
                Ok(out)
            }
            png::ColorType::Grayscale => {
                let mut out = Vec::with_capacity(raw.len() * 4);
                for &v in raw {
                    out.extend_from_slice(&[v, v, v, 255]);
                }
                Ok(out)
            }
            png::ColorType::GrayscaleAlpha => {
                let mut out = Vec::with_capacity((raw.len() / 2) * 4);
                for px in raw.chunks_exact(2) {
                    out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
                }
                Ok(out)
            }
            other => Err(format!("unsupported PNG color type for splatmap {path}: {other:?}")),
        },
        _ => Err(format!("unsupported target channel count: {target_channels}")),
    }?;

    Ok(resize_nearest_u8(&channels_converted, src_size, target_size, target_channels))
}

/// Resizes the input image data using nearest neighbor scaling to the specified target size. The function takes into account the number of channels in the image data to ensure that the output is correctly formatted for use in terrain rendering.
/// 
/// # Arguments
/// * `data` - The raw pixel data of the source image as a byte slice.
/// * `src_size` - The dimensions of the source image (width, height).
/// * `dst_size` - The desired dimensions of the output image (width, height).
/// * `channels` - The number of channels in the image data (e.g., 1 for heightmap, 4 for splatmap).
/// 
/// # Returns
/// A vector of bytes containing the resized image data, formatted according to the specified number of channels
fn resize_nearest_u8(data: &[u8], src_size: (u32, u32), dst_size: (u32, u32), channels: usize) -> Vec<u8> {
    let (src_w, src_h) = src_size;
    let (dst_w, dst_h) = dst_size;
    if src_w == dst_w && src_h == dst_h {
        return data.to_vec();
    }

    let mut out = vec![0u8; (dst_w as usize) * (dst_h as usize) * channels];
    for y in 0..dst_h {
        let sy = (y * src_h / dst_h) as usize;
        for x in 0..dst_w {
            let sx = (x * src_w / dst_w) as usize;
            let src_idx = (sy * src_w as usize + sx) * channels;
            let dst_idx = (y as usize * dst_w as usize + x as usize) * channels;
            out[dst_idx..dst_idx + channels].copy_from_slice(&data[src_idx..src_idx + channels]);
        }
    }
    out
}
