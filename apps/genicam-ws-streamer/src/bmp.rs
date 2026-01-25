//! Minimal BMP encoder for 8-bit grayscale (Mono8).
//!
//! The encoder emits a full BMP file:
//! - BITMAPFILEHEADER (14 bytes)
//! - BITMAPINFOHEADER (40 bytes)
//! - 256-entry grayscale palette (BGRA)
//! - pixel data with 4-byte row alignment
//!
//! We use a *top-down* DIB by storing a negative height in the header.
//! That lets us write rows in the same order as the incoming buffer
//! (top row first) without flipping the image.

use bytes::Bytes;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BmpError {
    #[error("pixel buffer size mismatch: expected {expected} bytes, got {actual}")]
    SizeMismatch { expected: usize, actual: usize },
    #[error("pixel buffer size overflow for width*height")]
    SizeOverflow,
}

#[derive(Debug, Clone)]
pub struct BmpEncoder {
    pub width: u32,
    pub height: u32,
    pub header_prefix: Vec<u8>,
    pub row_stride: usize,
    pub image_size: usize,
}

impl BmpEncoder {
    pub fn new(width: u32, height: u32) -> Self {
        // 8-bit BMP rows are padded to 4-byte alignment.
        let row_stride = ((width as usize + 3) / 4) * 4;
        let image_size = row_stride * height as usize;

        let palette_size = 256 * 4;
        let header_size = 14 + 40 + palette_size;
        let file_size = header_size + image_size;

        let mut header_prefix = Vec::with_capacity(header_size);

        // BITMAPFILEHEADER (14 bytes)
        header_prefix.extend_from_slice(b"BM");
        header_prefix.extend_from_slice(&(file_size as u32).to_le_bytes());
        header_prefix.extend_from_slice(&0u16.to_le_bytes()); // reserved1
        header_prefix.extend_from_slice(&0u16.to_le_bytes()); // reserved2
        header_prefix.extend_from_slice(&(header_size as u32).to_le_bytes()); // pixel data offset

        // BITMAPINFOHEADER (40 bytes)
        header_prefix.extend_from_slice(&(40u32).to_le_bytes()); // header size
        header_prefix.extend_from_slice(&(width as i32).to_le_bytes());
        // Negative height => top-down DIB (no row flip needed).
        header_prefix.extend_from_slice(&(-(height as i32)).to_le_bytes());
        header_prefix.extend_from_slice(&(1u16).to_le_bytes()); // planes
        header_prefix.extend_from_slice(&(8u16).to_le_bytes()); // bits per pixel
        header_prefix.extend_from_slice(&(0u32).to_le_bytes()); // compression (BI_RGB)
        header_prefix.extend_from_slice(&(image_size as u32).to_le_bytes());
        header_prefix.extend_from_slice(&0u32.to_le_bytes()); // x pixels per meter
        header_prefix.extend_from_slice(&0u32.to_le_bytes()); // y pixels per meter
        header_prefix.extend_from_slice(&(256u32).to_le_bytes()); // colors used
        header_prefix.extend_from_slice(&0u32.to_le_bytes()); // important colors

        // 256-entry grayscale palette (B, G, R, A=0)
        for value in 0u8..=255 {
            header_prefix.push(value);
            header_prefix.push(value);
            header_prefix.push(value);
            header_prefix.push(0);
        }

        Self {
            width,
            height,
            header_prefix,
            row_stride,
            image_size,
        }
    }

    pub fn encode_gray8(&self, pixels: &[u8]) -> Result<Bytes, BmpError> {
        let expected = (self.width as usize)
            .checked_mul(self.height as usize)
            .ok_or(BmpError::SizeOverflow)?;

        if pixels.len() != expected {
            return Err(BmpError::SizeMismatch {
                expected,
                actual: pixels.len(),
            });
        }

        let mut buffer = Vec::with_capacity(self.header_prefix.len() + self.image_size);
        buffer.extend_from_slice(&self.header_prefix);

        let row_width = self.width as usize;
        let padding = self.row_stride.saturating_sub(row_width);
        let pad_bytes = [0u8; 4];

        for row in 0..self.height as usize {
            let start = row * row_width;
            let end = start + row_width;
            buffer.extend_from_slice(&pixels[start..end]);
            if padding > 0 {
                buffer.extend_from_slice(&pad_bytes[..padding]);
            }
        }

        Ok(Bytes::from(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::BmpEncoder;

    #[test]
    fn encode_2x2_has_valid_headers_and_palette() {
        let encoder = BmpEncoder::new(2, 2);
        let pixels = [0u8, 127, 255, 64];
        let bytes = encoder.encode_gray8(&pixels).expect("encode succeeds");

        // BMP signature
        assert_eq!(&bytes[0..2], b"BM");

        // File size matches buffer length.
        let file_size = u32::from_le_bytes(bytes[2..6].try_into().expect("file size"));
        assert_eq!(file_size as usize, bytes.len());

        // Pixel data offset should include 14 + 40 + 256*4 bytes.
        let offset = u32::from_le_bytes(bytes[10..14].try_into().expect("offset"));
        assert_eq!(offset, 14 + 40 + 256 * 4);

        // Palette sanity check: first and last entries are grayscale.
        let palette_start = 14 + 40;
        let palette_end = palette_start + 256 * 4;
        assert!(bytes.len() >= palette_end);
        assert_eq!(&bytes[palette_start..palette_start + 4], &[0, 0, 0, 0]);
        assert_eq!(&bytes[palette_end - 4..palette_end], &[255, 255, 255, 0]);

        // Pixel data is top-down and padded to 4-byte rows (row_stride = 4).
        let data = &bytes[offset as usize..];
        assert_eq!(&data[0..2], &pixels[0..2]); // first row
        assert_eq!(&data[4..6], &pixels[2..4]); // second row (after padding)
    }
}
