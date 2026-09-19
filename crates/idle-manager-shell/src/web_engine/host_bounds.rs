//! The pure geometry behind `EngineHost`'s bounds, kept dependency-free so it
//! can be unit-tested on Linux the way `profile_name.rs` is (architecture
//! rule 14) — `make windows-check` only compiles and lints the Windows
//! target, it never links or runs it.

/// A logical rectangle in window coordinates: an origin and a size, both in
/// the same units `wry::Rect` and `gdk::graphene::Rect` already use, so the
/// caller converts at the boundary rather than this module taking on either
/// crate as a dependency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bounds {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

/// `EngineHost`'s bounds in window coordinates: the widget's own bounds
/// within its native window (`compute_bounds` against the window's root),
/// shifted by the toplevel surface's own transform (`Native::surface_transform`,
/// non-zero on a client-side-decorated Wayland window). The widget's size
/// passes through unchanged; only its origin moves.
///
/// A pure function of the two rectangles gtk4-rs already computes safely, so
/// `EngineHost`'s `imp.rs` owns nothing but calling it on every allocation and
/// on `notify::scale-factor` (roadmap item 12, "Airspace").
pub(crate) fn place_bounds(widget_bounds: Bounds, surface_transform: Bounds) -> Bounds {
    Bounds {
        x: widget_bounds.x + surface_transform.x,
        y: widget_bounds.y + surface_transform.y,
        width: widget_bounds.width,
        height: widget_bounds.height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_bounds_adds_the_surface_transform_to_the_widget_origin() {
        let widget = Bounds {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 200.0,
        };
        let transform = Bounds {
            x: 5.0,
            y: 8.0,
            width: 0.0,
            height: 0.0,
        };

        let placed = place_bounds(widget, transform);

        assert_eq!(
            (placed.x, placed.y, placed.width, placed.height),
            (15.0, 28.0, 300.0, 200.0),
        );
    }

    #[test]
    fn place_bounds_keeps_the_widgets_own_size() {
        let widget = Bounds {
            x: 0.0,
            y: 0.0,
            width: 640.0,
            height: 480.0,
        };
        let transform = Bounds {
            x: 100.0,
            y: 100.0,
            width: 0.0,
            height: 0.0,
        };

        let placed = place_bounds(widget, transform);

        assert_eq!((placed.width, placed.height), (640.0, 480.0));
    }

    #[test]
    fn an_origin_outside_the_window_stays_outside() {
        let widget = Bounds {
            x: -50.0,
            y: -50.0,
            width: 100.0,
            height: 100.0,
        };
        let transform = Bounds {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };

        let placed = place_bounds(widget, transform);

        assert!(placed.x < 0.0 && placed.y < 0.0);
    }
}
