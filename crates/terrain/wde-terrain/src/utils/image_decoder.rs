use std::{fs::File, io::BufReader};

/// Decoded image: values of each channel normalized to [0, 1], row-major.
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub channels: usize,
    pub data: Vec<f32>
}

impl DecodedImage {
    pub fn get(&self, x: u32, y: u32, channel: usize) -> f32 {
        let channel = channel.min(self.channels - 1);
        self.data[(y as usize * self.width as usize + x as usize) * self.channels + channel]
    }

    /// Channel `channel` bilinearly resampled to `size` x `size` samples, corners aligned (the
    /// first and last samples stay on the image edges).
    pub fn resample(&self, channel: usize, size: u32) -> Vec<f32> {
        if self.width == size && self.height == size {
            let channel = channel.min(self.channels - 1);
            return self
                .data
                .iter()
                .skip(channel)
                .step_by(self.channels)
                .copied()
                .collect();
        }
        let scale = |n: u32| (n.max(2) - 1) as f32 / (size - 1) as f32;
        let (sx, sy) = (scale(self.width), scale(self.height));
        let mut out = Vec::with_capacity((size * size) as usize);
        for y in 0..size {
            let fy = y as f32 * sy;
            let (y0, ty) = ((fy.floor() as u32).min(self.height - 1), fy.fract());
            let y1 = (y0 + 1).min(self.height - 1);
            for x in 0..size {
                let fx = x as f32 * sx;
                let (x0, tx) = ((fx.floor() as u32).min(self.width - 1), fx.fract());
                let x1 = (x0 + 1).min(self.width - 1);
                let top = self.get(x0, y0, channel) * (1.0 - tx) + self.get(x1, y0, channel) * tx;
                let bottom =
                    self.get(x0, y1, channel) * (1.0 - tx) + self.get(x1, y1, channel) * tx;
                out.push(top * (1.0 - ty) + bottom * ty);
            }
        }
        out
    }
}

/// Decodes a PNG (grayscale or color, 8 or 16 bits per channel) into normalized values.
pub fn decode_png(path: &str) -> Result<DecodedImage, String> {
    let mut decoder = png::Decoder::new(BufReader::new(
        File::open(path).map_err(|e| format!("failed to open {path}: {e}"))?
    ));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder
        .read_info()
        .map_err(|e| format!("failed to read PNG header for {path}: {e}"))?;
    let mut raw = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| format!("invalid output buffer size for {path}"))?
    ];
    let info = reader
        .next_frame(&mut raw)
        .map_err(|e| format!("failed to decode PNG frame for {path}: {e}"))?;
    let raw = &raw[..info.buffer_size()];
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => return Err(format!("unsupported PNG color type for {path}: {other:?}"))
    };
    let data = match info.bit_depth {
        png::BitDepth::Sixteen => raw
            .chunks_exact(2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]) as f32 / u16::MAX as f32)
            .collect(),
        _ => raw.iter().map(|&b| b as f32 / u8::MAX as f32).collect()
    };
    Ok(DecodedImage {
        width: info.width,
        height: info.height,
        channels,
        data
    })
}
