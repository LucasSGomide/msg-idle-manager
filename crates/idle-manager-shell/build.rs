//! Compiles `resources/` into a `GResource` bundle linked into the crate, so the
//! UI templates ship inside the binary with no loose files to install beside
//! it (architecture rule 13).

fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/idle-manager.gresource.xml",
        "idle-manager.gresource",
    );
}
