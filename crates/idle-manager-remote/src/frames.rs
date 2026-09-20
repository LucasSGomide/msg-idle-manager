//! Pictures for the phone: a captured [`Frame`] becomes one binary message of
//! two big-endian `u32`s, width then height, followed by a JPEG.
//!
//! Raw pixels are encoded here at [`JPEG_QUALITY`] and a frame whose encoded
//! bytes hash the same as the last one sent is skipped, so a still page costs
//! nothing on the wire. All of it runs on the connection thread, never on the
//! GTK main context (architecture rule 10).

use std::hash::{DefaultHasher, Hasher as _};

use idle_manager_core::Frame;
use jpeg_encoder::{ColorType, Encoder};

/// The JPEG quality frames are encoded at: the point where a 412 × 915 page
/// encodes in single-digit milliseconds and text stays legible.
pub(crate) const JPEG_QUALITY: u8 = 75;

/// The bytes before the JPEG in a binary message: width and height, each a
/// big-endian `u32`.
pub(crate) const HEADER_LEN: usize = 8;

/// Why a frame could not be turned into a message.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FrameError {
    /// A side exceeds what JPEG can describe.
    #[error("a {width} × {height} frame does not fit a JPEG's 16-bit sides")]
    TooLarge { width: u32, height: u32 },
    /// The pixel buffer is shorter than `stride × height`.
    #[error("the pixel buffer holds {length} bytes but {stride} × {height} needs {needed}")]
    ShortBuffer {
        length: usize,
        stride: u32,
        height: u32,
        needed: usize,
    },
    /// A ready-made JPEG has no frame header naming its size.
    #[error("the JPEG carries no start-of-frame segment")]
    NoDimensions,
    /// The encoder refused the pixels.
    #[error("JPEG encoding failed: {reason}")]
    Encoding { reason: String },
}

/// A frame encoded and ready to frame as a binary message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Encoded {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) jpeg: Vec<u8>,
}

impl Encoded {
    /// The binary message payload: the size header then the JPEG.
    pub(crate) fn message(&self) -> Vec<u8> {
        let mut message = Vec::with_capacity(HEADER_LEN + self.jpeg.len());
        message.extend_from_slice(&self.width.to_be_bytes());
        message.extend_from_slice(&self.height.to_be_bytes());
        message.extend_from_slice(&self.jpeg);
        message
    }
}

/// Encodes `frame`: raw pixels through the JPEG encoder, a JPEG passed
/// through with its size read from its own header.
///
/// # Errors
///
/// [`FrameError`] for a frame the format cannot carry or a buffer that does
/// not match its own dimensions.
pub(crate) fn encode(frame: Frame) -> Result<Encoded, FrameError> {
    match frame {
        Frame::Jpeg(jpeg) => {
            let (width, height) = jpeg_dimensions(&jpeg).ok_or(FrameError::NoDimensions)?;
            Ok(Encoded {
                width,
                height,
                jpeg,
            })
        }
        Frame::Rgba {
            width,
            height,
            stride,
            bytes,
        } => encode_rgba(width, height, stride, &bytes),
    }
}

fn encode_rgba(width: u32, height: u32, stride: u32, bytes: &[u8]) -> Result<Encoded, FrameError> {
    let (Ok(jpeg_width), Ok(jpeg_height)) = (u16::try_from(width), u16::try_from(height)) else {
        return Err(FrameError::TooLarge { width, height });
    };
    let needed = stride as usize * height as usize;
    if bytes.len() < needed {
        return Err(FrameError::ShortBuffer {
            length: bytes.len(),
            stride,
            height,
            needed,
        });
    }
    let row_bytes = width as usize * 4;
    let packed: Vec<u8>;
    let pixels = if stride as usize == row_bytes {
        &bytes[..needed]
    } else {
        packed = bytes
            .chunks_exact(stride as usize)
            .take(height as usize)
            .flat_map(|row| &row[..row_bytes])
            .copied()
            .collect();
        &packed
    };
    let mut jpeg = Vec::new();
    Encoder::new(&mut jpeg, JPEG_QUALITY)
        .encode(pixels, jpeg_width, jpeg_height, ColorType::Rgba)
        .map_err(|error| FrameError::Encoding {
            reason: error.to_string(),
        })?;
    Ok(Encoded {
        width,
        height,
        jpeg,
    })
}

/// Width and height from a JPEG's start-of-frame segment, or `None` when the
/// bytes carry none.
pub(crate) fn jpeg_dimensions(jpeg: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    while at + 4 <= jpeg.len() {
        if jpeg[at] != 0xFF {
            return None;
        }
        let marker = jpeg[at + 1];
        if marker == 0xFF {
            at += 1;
            continue;
        }
        let length = usize::from(u16::from_be_bytes([jpeg[at + 2], jpeg[at + 3]]));
        if is_start_of_frame(marker) {
            let segment = jpeg.get(at + 5..at + 9)?;
            let height = u32::from(u16::from_be_bytes([segment[0], segment[1]]));
            let width = u32::from(u16::from_be_bytes([segment[2], segment[3]]));
            return Some((width, height));
        }
        at += 2 + length;
    }
    None
}

/// SOF0 to SOF15 are `C0`–`CF` except `C4` (DHT), `C8` (JPG) and `CC` (DAC).
fn is_start_of_frame(marker: u8) -> bool {
    matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC)
}

/// Remembers the last JPEG sent, so an identical one is not sent again.
#[derive(Debug, Default)]
pub(crate) struct Dedup {
    last_hash: Option<u64>,
}

impl Dedup {
    /// Whether `jpeg` differs from the last one admitted; records it if so.
    pub(crate) fn admit(&mut self, jpeg: &[u8]) -> bool {
        let mut hasher = DefaultHasher::new();
        hasher.write(jpeg);
        let hash = hasher.finish();
        if self.last_hash == Some(hash) {
            return false;
        }
        self.last_hash = Some(hash);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_rgba(width: u32, height: u32, stride: u32, rgba: [u8; 4]) -> Frame {
        let mut bytes = vec![0u8; stride as usize * height as usize];
        for row in bytes.chunks_exact_mut(stride as usize) {
            let (pixels, _) = row[..width as usize * 4].as_chunks_mut::<4>();
            for pixel in pixels {
                *pixel = rgba;
            }
        }
        Frame::Rgba {
            width,
            height,
            stride,
            bytes,
        }
    }

    #[test]
    fn the_message_starts_with_width_and_height_big_endian_then_the_jpeg() {
        let encoded = encode(solid_rgba(16, 8, 64, [200, 30, 30, 255])).expect("encodes");

        let message = encoded.message();

        assert_eq!(
            (&message[..HEADER_LEN], &message[HEADER_LEN..HEADER_LEN + 2]),
            (&[0, 0, 0, 16, 0, 0, 0, 8][..], &[0xFF, 0xD8][..])
        );
    }

    #[test]
    fn the_jpeg_names_the_frames_own_size() {
        let encoded = encode(solid_rgba(20, 12, 80, [0, 0, 255, 255])).expect("encodes");

        assert_eq!(jpeg_dimensions(&encoded.jpeg), Some((20, 12)));
    }

    #[test]
    fn a_padded_stride_encodes_the_same_picture_as_a_packed_one() {
        let packed = encode(solid_rgba(16, 8, 64, [10, 200, 10, 255])).expect("encodes");

        let padded = encode(solid_rgba(16, 8, 96, [10, 200, 10, 255])).expect("encodes");

        assert_eq!(padded, packed);
    }

    #[test]
    fn a_ready_jpeg_passes_through_with_its_own_dimensions() {
        let encoded = encode(solid_rgba(16, 8, 64, [1, 2, 3, 255])).expect("encodes");

        let passed = encode(Frame::Jpeg(encoded.jpeg.clone())).expect("passes through");

        assert_eq!(passed, encoded);
    }

    #[test]
    fn a_short_pixel_buffer_is_refused_by_its_numbers() {
        let error = encode(Frame::Rgba {
            width: 4,
            height: 4,
            stride: 16,
            bytes: vec![0; 10],
        })
        .expect_err("too short");

        assert!(matches!(error, FrameError::ShortBuffer { needed: 64, .. }));
    }

    #[test]
    fn the_same_bytes_are_admitted_once() {
        let mut dedup = Dedup::default();

        let admitted = [
            dedup.admit(b"same"),
            dedup.admit(b"same"),
            dedup.admit(b"other"),
        ];

        assert_eq!(admitted, [true, false, true]);
    }
}
