//! The enrolment address as a picture the phone's camera reads: the `qrcode`
//! crate's modules rasterised into an opaque black-on-white
//! [`gdk::MemoryTexture`], with no image library in between.

use gtk::gdk;
use gtk::glib;
use gtk4 as gtk;
use qrcode::{Color, QrCode};

/// Light modules on every side of the code — the quiet zone ISO 18004 asks
/// for, without which a camera struggles to find the code's edge.
const QUIET_ZONE_MODULES: usize = 4;

/// The longest side the picture is drawn at, in pixels. The module scale is
/// the largest whole number that keeps the code inside it, so every module is
/// a crisp square and the picture is never resampled.
const MAX_SIDE_PX: usize = 260;

/// Bytes per pixel in the `R8G8B8A8` layout the texture is built from.
const BYTES_PER_PIXEL: usize = 4;

/// A dark module's pixel.
const DARK: [u8; BYTES_PER_PIXEL] = [0x00, 0x00, 0x00, 0xff];

/// A light module's pixel — opaque, so the code stays readable on a dark
/// theme's window background.
const LIGHT: [u8; BYTES_PER_PIXEL] = [0xff, 0xff, 0xff, 0xff];

/// The code drawn as pixels: a square `side` px on each edge, rows top to
/// bottom, `side × 4` bytes per row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Raster {
    side: usize,
    pixels: Vec<u8>,
}

#[cfg(test)]
impl Raster {
    /// The picture's width and height in pixels.
    fn side(&self) -> usize {
        self.side
    }

    /// The pixel at column `x`, row `y`.
    fn pixel(&self, x: usize, y: usize) -> [u8; BYTES_PER_PIXEL] {
        let start = (y * self.side + x) * BYTES_PER_PIXEL;
        let mut pixel = [0; BYTES_PER_PIXEL];
        pixel.copy_from_slice(&self.pixels[start..start + BYTES_PER_PIXEL]);
        pixel
    }
}

/// Encodes `address` and draws it with its quiet zone at the largest whole
/// module scale that fits [`MAX_SIDE_PX`]. `None` only when the address is
/// too long for any QR version, which a server-minted enrolment address
/// never is.
pub(super) fn raster(address: &str) -> Option<Raster> {
    let code = QrCode::new(address).ok()?;
    let modules = code.width();
    let colors = code.to_colors();
    let side_modules = modules + 2 * QUIET_ZONE_MODULES;
    let scale = (MAX_SIDE_PX / side_modules).max(1);
    let side = side_modules * scale;

    let mut pixels = Vec::with_capacity(side * side * BYTES_PER_PIXEL);
    for y in 0..side {
        for x in 0..side {
            pixels.extend_from_slice(&pixel_at(&colors, modules, scale, x, y));
        }
    }
    Some(Raster { side, pixels })
}

/// The pixel at (`x`, `y`) of the scaled picture: light in the quiet zone,
/// otherwise the module it falls inside.
fn pixel_at(
    colors: &[Color],
    modules: usize,
    scale: usize,
    x: usize,
    y: usize,
) -> [u8; BYTES_PER_PIXEL] {
    let column = (x / scale).checked_sub(QUIET_ZONE_MODULES);
    let row = (y / scale).checked_sub(QUIET_ZONE_MODULES);
    match (column, row) {
        (Some(column), Some(row)) if column < modules && row < modules => {
            match colors[row * modules + column] {
                Color::Dark => DARK,
                Color::Light => LIGHT,
            }
        }
        _ => LIGHT,
    }
}

/// [`raster`] handed to GDK as a texture a `gtk::Picture` paints at its own
/// size.
pub(super) fn texture(address: &str) -> Option<gdk::MemoryTexture> {
    let raster = raster(address)?;
    let side = i32::try_from(raster.side).ok()?;
    let stride = raster.side * BYTES_PER_PIXEL;
    let bytes = glib::Bytes::from_owned(raster.pixels);
    Some(gdk::MemoryTexture::new(
        side,
        side,
        gdk::MemoryFormat::R8g8b8a8,
        &bytes,
        stride,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of a real offer: the server's `http://<mesh ip>:<port>/enrol/`
    /// followed by a 64-hex-digit code.
    const ADDRESS: &str = "http://100.101.12.7:7466/enrol/3fa9c1e2b8d4f6a0c5e7d9b1a3f5c7e9d2b4a6c8e0f1a3b5c7d9e1f3a5b7c9d1";

    #[test]
    fn the_picture_is_the_code_plus_its_quiet_zone_at_a_whole_module_scale() {
        let modules = QrCode::new(ADDRESS).expect("the address encodes").width();

        let raster = raster(ADDRESS).expect("the address encodes");

        let side_modules = modules + 2 * QUIET_ZONE_MODULES;
        let scale = MAX_SIDE_PX / side_modules;
        assert_eq!(
            (raster.side(), raster.pixels.len()),
            (
                side_modules * scale,
                side_modules * scale * side_modules * scale * BYTES_PER_PIXEL
            )
        );
    }

    #[test]
    fn the_picture_fits_the_largest_side() {
        let raster = raster(ADDRESS).expect("the address encodes");

        assert!(raster.side() <= MAX_SIDE_PX);
    }

    #[test]
    fn the_quiet_zone_is_light_and_the_finder_pattern_corner_is_dark() {
        let raster = raster(ADDRESS).expect("the address encodes");
        let modules = QrCode::new(ADDRESS).expect("the address encodes").width();
        let scale = MAX_SIDE_PX / (modules + 2 * QUIET_ZONE_MODULES);
        let first_module = QUIET_ZONE_MODULES * scale;

        let corners = [
            raster.pixel(0, 0),
            raster.pixel(raster.side() - 1, raster.side() - 1),
            raster.pixel(first_module, first_module),
        ];

        assert_eq!(corners, [LIGHT, LIGHT, DARK]);
    }

    #[test]
    fn every_pixel_is_opaque_black_or_white() {
        let raster = raster(ADDRESS).expect("the address encodes");

        let all_pure = raster
            .pixels
            .as_chunks::<BYTES_PER_PIXEL>()
            .0
            .iter()
            .all(|pixel| *pixel == DARK || *pixel == LIGHT);

        assert!(all_pure);
    }
}
