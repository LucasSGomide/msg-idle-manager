//! Composition root: reads configuration, builds one adapter per port, hands
//! them to the shell, and runs the GTK application.

fn main() {
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
}
