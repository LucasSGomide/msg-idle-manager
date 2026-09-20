//! The phone page's pure JavaScript — its SHA-256 and HMAC, the tap/scroll
//! classifier and the canvas-to-frame mapping — lifted out from between the
//! page's `// BEGIN pure` / `// END pure` markers and run under `node`, so a
//! broken proof or a mis-scaled tap is caught here rather than on a phone.
//!
//! Every test passes with a printed note when no `node` can be found, so a
//! machine without it still runs the rest of the suite (code-standards rule
//! 25 in spirit: no browser and no display server are required).

use std::path::PathBuf;
use std::process::Command;

const PAGE: &str = include_str!("../assets/phone.html");
const BEGIN_MARKER: &str = "// BEGIN pure";
const END_MARKER: &str = "// END pure";

/// The script between the markers: the page's pure functions and nothing else.
fn pure_block() -> &'static str {
    let start = PAGE
        .find(BEGIN_MARKER)
        .expect("the page marks where its pure block begins");
    let end = PAGE
        .find(END_MARKER)
        .expect("the page marks where its pure block ends");
    &PAGE[start..end]
}

/// `node` from `PATH`, else the newest one nvm installed for this user.
fn node() -> Option<PathBuf> {
    if Command::new("node").arg("--version").output().is_ok() {
        return Some(PathBuf::from("node"));
    }
    let home = std::env::var_os("HOME")?;
    let versions = PathBuf::from(home).join(".nvm/versions/node");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(versions)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("bin/node"))
        .filter(|path| path.is_file())
        .collect();
    candidates.sort();
    candidates.pop()
}

/// Runs `check` after the pure block under node and returns what it printed,
/// or `None` with a note when node is absent.
fn run_pure(name: &str, check: &str) -> Option<String> {
    let Some(node) = node() else {
        eprintln!("note: `node` is not on PATH and nvm holds none; {name} is skipped");
        return None;
    };
    let script = format!(
        "var window = {{}};\n{}\nvar p = window.__phone;\n{check}\n",
        pure_block()
    );
    let path = std::env::temp_dir().join(format!(
        "idle-manager-phone-{}-{name}.js",
        std::process::id()
    ));
    std::fs::write(&path, script).expect("the script is written to the temp dir");
    let output = Command::new(node)
        .arg(&path)
        .output()
        .expect("node runs the script");
    std::fs::remove_file(&path).ok();
    assert!(
        output.status.success(),
        "node failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[test]
fn the_inline_sha256_hashes_abc_to_the_known_digest() {
    let Some(digest) = run_pure(
        "sha256",
        "console.log(p.bytesToHex(p.sha256(p.utf8('abc'))));",
    ) else {
        return;
    };

    assert_eq!(
        digest,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn the_inline_hmac_matches_rfc_4231_test_case_2() {
    let Some(mac) = run_pure(
        "hmac",
        "console.log(p.bytesToHex(p.hmacSha256(p.utf8('Jefe'), p.utf8('what do ya want for nothing?'))));",
    ) else {
        return;
    };

    assert_eq!(
        mac,
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

#[test]
fn the_phone_proof_is_the_hmac_over_the_prefixed_challenge() {
    let Some(proof) = run_pure(
        "proof",
        "console.log(p.proof('4a656665', 'phone|', 'abc'), p.bytesToHex(p.hmacSha256(p.utf8('Jefe'), p.utf8('phone|abc'))));",
    ) else {
        return;
    };

    let mut halves = proof.split(' ');
    assert_eq!(halves.next(), halves.next());
}

#[test]
fn a_short_still_press_is_a_tap_and_a_long_move_is_a_scroll_with_its_deltas() {
    let Some(classified) = run_pure(
        "classifier",
        "var tap = p.endPress(p.beginPress(10, 10, 0), 13, 12, 200);\n\
         var moved = p.movePress(p.beginPress(10, 10, 0), 50, 10, 100);\n\
         console.log(JSON.stringify([tap, moved.scroll, p.endPress(moved.press, 50, 10, 120)]));",
    ) else {
        return;
    };

    assert_eq!(classified, r#"["tap",{"dx":40,"dy":0},"scroll"]"#);
}

#[test]
fn a_canvas_point_maps_back_through_the_draw_scale_into_frame_pixels() {
    let Some(mapped) = run_pure(
        "mapping",
        "var fit = p.fitFrame(200, 400, 400, 800);\n\
         console.log(JSON.stringify([p.toViewport({x: 200, y: 300}, fit), p.toViewportDelta(40, -20, fit)]));",
    ) else {
        return;
    };

    assert_eq!(mapped, r#"[{"x":100,"y":150},{"dx":20,"dy":-10}]"#);
}

#[test]
fn a_binary_message_yields_its_width_height_and_the_jpeg_offset() {
    let Some(header) = run_pure(
        "header",
        "console.log(JSON.stringify(p.parseFrameHeader(new Uint8Array([0,0,1,0x9c,0,0,3,0x93,0xff,0xd8]).buffer)));",
    ) else {
        return;
    };

    assert_eq!(header, r#"{"width":412,"height":915,"jpegOffset":8}"#);
}

#[test]
fn the_shown_workspace_is_the_one_holding_current_else_the_first() {
    let Some(shown) = run_pure(
        "workspace",
        "var workspaces = [{name: 'A', accounts: [{id: 'a'}]}, {name: 'B', accounts: [{id: 'b'}]}];\n\
         console.log(p.shownWorkspace({current: 'b', workspaces: workspaces}).name, \
         p.shownWorkspace({current: null, workspaces: workspaces}).name, \
         p.shownWorkspace({current: null, workspaces: []}));",
    ) else {
        return;
    };

    assert_eq!(shown, "B A null");
}
