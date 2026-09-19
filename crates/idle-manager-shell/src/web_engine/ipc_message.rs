//! Parses the JSON `resources/js/webview2-bridge.js` posts through
//! `window.ipc.postMessage` (roadmap item 12 task 03) into the one of two
//! actions the shell understands.
//!
//! Pure and dependency-free of anything Windows-specific on purpose, same
//! reasoning as `virtual_key.rs`: `make windows-check` only compiles and
//! lints the Windows target, it never runs what it compiles, so this is the
//! one part of the bridge this machine can actually execute a test against.

use serde::Deserialize;

/// One message the bridge script posted, decoded from
/// `{"handler": ..., "body": ...}`.
#[derive(Debug, Deserialize)]
struct RawMessage {
    handler: String,
    body: serde_json::Value,
}

/// One page-console entry — the same three fields
/// `web_engine/webkit.rs::register_page_console_handler` reads off `WebKit`'s
/// own message object on Linux, so both backends log through the one shape.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct PageConsoleEntry {
    pub(crate) level: String,
    pub(crate) origin: String,
    pub(crate) text: String,
}

/// What one parsed ipc message means to the shell.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum IpcAction {
    /// `Ctrl`+wheel over the page (`FR.11.3`): the sign of the wheel event's
    /// `deltaY` the bridge script's own listener already resolved to `±1` —
    /// negative is a notch up, matching `window/imp.rs`'s
    /// `connect_zoom_scrolled` convention on Linux exactly, so the caller
    /// applies the same `< 0.0` comparison rather than a second, possibly
    /// different rule.
    ZoomStep(i64),
    /// A page's forwarded `console.*` call or uncaught error.
    PageConsole(PageConsoleEntry),
}

/// Parses one raw ipc string into the action it names, or `None` for
/// malformed JSON or a handler this shell does not recognise — logged by the
/// caller, never a panic (code standards rules 14, 15).
pub(crate) fn parse_ipc_message(raw: &str) -> Option<IpcAction> {
    let message: RawMessage = serde_json::from_str(raw).ok()?;
    match message.handler.as_str() {
        "zoomStep" => message.body.as_i64().map(IpcAction::ZoomStep),
        "pageConsole" => serde_json::from_value(message.body)
            .ok()
            .map(IpcAction::PageConsole),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zoom_step_message_maps_to_its_signed_step() {
        let action = parse_ipc_message(r#"{"handler":"zoomStep","body":-1}"#);

        assert_eq!(action, Some(IpcAction::ZoomStep(-1)));
    }

    #[test]
    fn a_page_console_message_maps_to_a_console_entry() {
        let action = parse_ipc_message(
            r#"{"handler":"pageConsole","body":{"level":"warn","origin":"https://example.test","text":"hi"}}"#,
        );

        assert_eq!(
            action,
            Some(IpcAction::PageConsole(PageConsoleEntry {
                level: "warn".to_owned(),
                origin: "https://example.test".to_owned(),
                text: "hi".to_owned(),
            }))
        );
    }

    #[test]
    fn malformed_json_yields_no_action_rather_than_a_panic() {
        assert_eq!(parse_ipc_message("not json"), None);
    }

    #[test]
    fn an_unrecognised_handler_yields_no_action() {
        assert_eq!(
            parse_ipc_message(r#"{"handler":"unknown","body":null}"#),
            None
        );
    }
}
