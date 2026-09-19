//! The `IDLE_MANAGER_DUMP_FRAMES=<dir>` debug switch (roadmap item 13 task
//! 02): while it is set, every running account writes a picture of its page
//! into that folder every two seconds, so whether either engine returns a
//! *current* picture of a page while the window is minimised — the item's
//! first Blocker — is answered by files on disk rather than by guesswork.
//!
//! Off by default, like `FR.19.6`'s diagnostics, and never fatal: a folder
//! that cannot be written to is logged once and the dump stops.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gtk::gio;
use gtk::glib;
use gtk4 as gtk;

use idle_manager_core::SessionId;

use crate::web_engine::{CapturedFrame, EngineView};

/// Setting this in the environment to a folder turns the frame dump on.
const FRAME_DUMP_ENV: &str = "IDLE_MANAGER_DUMP_FRAMES";
/// How often one account's page is captured while the dump is on, in
/// seconds (code standards rule 5). Slow enough that a minute minimised
/// yields a readable handful of files rather than hundreds.
const FRAME_DUMP_INTERVAL_SECS: u64 = 2;
/// The largest sample value a binary `P6` file can carry per channel, which
/// is what eight-bit RGBA pixels already are.
const PPM_MAX_SAMPLE: u8 = u8::MAX;

/// The running dump for one view. Dropping it stops the timer at its next
/// tick: the timer holds a clone of the [`EngineView`] it captures from, so
/// a parked account's view lingers for at most one interval after its holder
/// lets go — acceptable for a debug switch, and exactly why this handle exists
/// rather than a fire-and-forget timer.
#[derive(Debug)]
pub(crate) struct FrameDump {
    stopped: Rc<Cell<bool>>,
}

impl Drop for FrameDump {
    fn drop(&mut self) {
        self.stopped.set(true);
    }
}

/// Arms the dump for `view` when [`FRAME_DUMP_ENV`] is set, or returns
/// `None` — the default — when it is not. The first capture runs
/// [`FRAME_DUMP_INTERVAL_SECS`] after the page's first paint; before that
/// there is nothing worth a picture.
pub(crate) fn arm(view: &EngineView, id: &SessionId) -> Option<FrameDump> {
    let directory = PathBuf::from(std::env::var_os(FRAME_DUMP_ENV)?);
    let stopped = Rc::new(Cell::new(false));
    tracing::debug!(session = %id, directory = %directory.display(), "frame dump armed");

    let view_for_paint = view.clone();
    let id = id.clone();
    let stopped_for_paint = Rc::clone(&stopped);
    view.connect_painted(move || {
        start_timer(&view_for_paint, &id, &directory, &stopped_for_paint);
    });

    Some(FrameDump { stopped })
}

/// Starts the repeating capture for `view`, writing frame `0`, `1`, `2`, …
/// until `stopped` is set — by the holder dropping its [`FrameDump`], or by
/// a write failing.
fn start_timer(view: &EngineView, id: &SessionId, directory: &Path, stopped: &Rc<Cell<bool>>) {
    let view = view.clone();
    let id = id.clone();
    let directory = directory.to_path_buf();
    let stopped = Rc::clone(stopped);
    let next_frame = Rc::new(Cell::new(0u32));

    glib::timeout_add_local(Duration::from_secs(FRAME_DUMP_INTERVAL_SECS), move || {
        if stopped.get() {
            return glib::ControlFlow::Break;
        }
        let frame_number = next_frame.get();
        next_frame.set(frame_number.wrapping_add(1));

        let id_for_capture = id.clone();
        let directory = directory.clone();
        let stopped = Rc::clone(&stopped);
        view.capture_frame(move |outcome| match outcome {
            Ok(frame) => write_frame(&directory, &id_for_capture, frame_number, frame, stopped),
            Err(error) => {
                tracing::warn!(session = %id_for_capture, %error, "frame capture failed");
            }
        });
        glib::ControlFlow::Continue
    });
}

/// Encodes `frame` and writes it to `<directory>/<id>-<n>.{ppm|jpg}` off the
/// main context (architecture rule 10), logging the write at `debug`. A
/// failed write sets `stopped`: a folder that refused one frame will refuse
/// the next, and one warning says so where a hundred would not.
fn write_frame(
    directory: &Path,
    id: &SessionId,
    frame_number: u32,
    frame: CapturedFrame,
    stopped: Rc<Cell<bool>>,
) {
    let path = directory.join(frame_file_name(id, frame_number, &frame));
    let bytes = encode(frame);
    let id = id.clone();

    glib::spawn_future_local(async move {
        let path_for_log = path.clone();
        let outcome =
            gio::spawn_blocking(move || std::fs::write(&path, &bytes).map(|()| bytes.len())).await;
        match outcome {
            Ok(Ok(written)) => tracing::debug!(
                session = %id,
                path = %path_for_log.display(),
                bytes = written,
                "frame written"
            ),
            Ok(Err(error)) => {
                tracing::warn!(
                    session = %id,
                    path = %path_for_log.display(),
                    %error,
                    "frame not written; the frame dump for this account stops here"
                );
                stopped.set(true);
            }
            Err(_) => tracing::warn!(session = %id, "the frame write did not complete"),
        }
    });
}

/// `<id>-<n>.ppm` for raw pixels, `<id>-<n>.jpg` for a JPEG.
fn frame_file_name(id: &SessionId, frame_number: u32, frame: &CapturedFrame) -> String {
    let extension = match frame {
        CapturedFrame::Rgba { .. } => "ppm",
        CapturedFrame::Jpeg(_) => "jpg",
    };
    format!("{id}-{frame_number}.{extension}")
}

/// The bytes to write for `frame`: a JPEG as it is, raw pixels as a binary
/// `P6` file ([`ppm_bytes`]).
fn encode(frame: CapturedFrame) -> Vec<u8> {
    match frame {
        CapturedFrame::Rgba {
            width,
            height,
            stride,
            bytes,
        } => ppm_bytes(width, height, stride, &bytes),
        CapturedFrame::Jpeg(bytes) => bytes,
    }
}

/// A binary `P6` portable pixmap of `width × height` RGBA pixels laid out
/// `stride` bytes per row: the header, then each row's `width` pixels with
/// the alpha byte dropped and the row padding skipped. `P6` because every
/// image viewer opens it and it needs no encoder — the point is to look at
/// the picture, not to ship it.
fn ppm_bytes(width: u32, height: u32, stride: u32, rgba: &[u8]) -> Vec<u8> {
    let row_length = (width as usize) * 4;
    let mut ppm = format!("P6\n{width} {height}\n{PPM_MAX_SAMPLE}\n").into_bytes();
    ppm.reserve((width as usize) * (height as usize) * 3);

    for row in rgba.chunks(stride as usize).take(height as usize) {
        let (pixels, _partial) = row.get(..row_length).unwrap_or(row).as_chunks::<4>();
        for [red, green, blue, _alpha] in pixels {
            ppm.extend_from_slice(&[*red, *green, *blue]);
        }
    }
    ppm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ppm_starts_with_the_p6_header_naming_the_size() {
        let ppm = ppm_bytes(2, 1, 8, &[0; 8]);

        assert!(ppm.starts_with(b"P6\n2 1\n255\n"));
    }

    #[test]
    fn a_ppm_drops_every_pixels_alpha_byte() {
        let ppm = ppm_bytes(2, 1, 8, &[1, 2, 3, 255, 4, 5, 6, 255]);

        let pixels = &ppm[b"P6\n2 1\n255\n".len()..];
        assert_eq!(pixels, &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn a_ppm_skips_the_padding_a_wide_stride_carries() {
        let rows = [[1, 1, 1, 0, 9, 9, 9, 9], [2, 2, 2, 0, 9, 9, 9, 9]].concat();

        let ppm = ppm_bytes(1, 2, 8, &rows);

        let pixels = &ppm[b"P6\n1 2\n255\n".len()..];
        assert_eq!(pixels, &[1, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn a_raw_frame_is_named_ppm_and_a_jpeg_frame_jpg() {
        let id = SessionId::new("session-0007");
        let raw = CapturedFrame::Rgba {
            width: 1,
            height: 1,
            stride: 4,
            bytes: vec![0; 4],
        };
        let jpeg = CapturedFrame::Jpeg(Vec::new());

        assert_eq!(frame_file_name(&id, 3, &raw), "session-0007-3.ppm");
        assert_eq!(frame_file_name(&id, 4, &jpeg), "session-0007-4.jpg");
    }

    #[test]
    fn a_jpeg_frame_is_written_as_it_is() {
        let bytes = vec![0xFF, 0xD8, 0xFF, 0xD9];

        assert_eq!(encode(CapturedFrame::Jpeg(bytes.clone())), bytes);
    }
}
