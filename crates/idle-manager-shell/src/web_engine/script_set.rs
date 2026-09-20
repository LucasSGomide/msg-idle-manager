//! The document-start scripts each engine injects into every page, and the
//! one line that arms or disarms the frame shim at run time.
//!
//! Pure on purpose: which scripts a view gets, in which order, and what
//! `set_watched` runs in the page are decisions both engines share and the
//! shell can unit-test on any machine — `make windows-check` only compiles
//! and lints the Windows target, it never runs it (architecture rule 14).
//! The Windows half is `cfg(any(windows, test))` for the same reason
//! `web_engine/profile_name.rs` is.

use crate::web_view::{HIDDEN_FRAME_INTERVAL_MS, WATCHED_FRAME_INTERVAL_MS};

/// The page-console bridge, compiled in rather than loaded from the
/// `GResource` bundle: the engines are its only callers, they need the source
/// before any widget exists, and `include_str!` keeps the read infallible so
/// no error path is needed for it.
pub(crate) const PAGE_CONSOLE_JS: &str = include_str!("../../resources/js/page-console.js");

/// The frame-callback shim (`FR.6.3`), brought in the same way and for the
/// same reason as [`PAGE_CONSOLE_JS`]. Injected into *every* page since
/// roadmap item 13 and dormant until armed — by [`KEEP_AWAKE_PRELUDE_JS`] for
/// a keep-awake account, or by [`watched_script`] while a phone watches.
pub(crate) const KEEP_AWAKE_JS: &str = include_str!("../../resources/js/keep-awake.js");

/// The one line a keep-awake account runs right after [`KEEP_AWAKE_JS`], before
/// the page itself: it arms the shim so the account behaves exactly as item 04
/// specified. Guarded, because a frame the shim did not reach must not throw
/// on the page's behalf.
pub(crate) const KEEP_AWAKE_PRELUDE_JS: &str =
    "if (window.__idleManager) window.__idleManager.setAwake(true);";

/// The `window.webkit.messageHandlers` shape `WebView2` lacks, defined on top
/// of `wry`'s `window.ipc` so [`PAGE_CONSOLE_JS`] and [`KEEP_AWAKE_JS`] run
/// unchanged on both engines (roadmap item 12, "The Windows scripts and
/// messages"). First in the Windows set so `window.webkit` exists by the time
/// they run.
#[cfg(any(windows, test))]
pub(crate) const WEBVIEW2_BRIDGE_JS: &str = include_str!("../../resources/js/webview2-bridge.js");

/// The document-start scripts a `WebKitGTK` view's content manager holds, in
/// injection order: the console bridge, the shim, and the arming prelude only
/// when `keep_awake` is on.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn webkit_script_set(keep_awake: bool) -> Vec<&'static str> {
    let mut scripts = vec![PAGE_CONSOLE_JS, KEEP_AWAKE_JS];
    if keep_awake {
        scripts.push(KEEP_AWAKE_PRELUDE_JS);
    }
    scripts
}

/// The initialization scripts a `WebView2` view is built with, in injection
/// order: [`WEBVIEW2_BRIDGE_JS`] first, then exactly [`webkit_script_set`]'s
/// scripts, so the two engines differ only by the bridge.
#[cfg(any(windows, test))]
pub(crate) fn webview2_script_set(keep_awake: bool) -> Vec<&'static str> {
    let mut scripts = vec![WEBVIEW2_BRIDGE_JS, PAGE_CONSOLE_JS, KEEP_AWAKE_JS];
    if keep_awake {
        scripts.push(KEEP_AWAKE_PRELUDE_JS);
    }
    scripts
}

/// The script `EngineView::set_watched` runs in a live page (`FR.4.3`): while
/// watched (`on`), the shim is armed and answers a hidden page every
/// [`WATCHED_FRAME_INTERVAL_MS`]; when the phone leaves, the account's own
/// `keep_awake` choice and [`HIDDEN_FRAME_INTERVAL_MS`] are put back — exactly
/// what the prelude would have set, so a keep-awake account is unchanged and
/// any other goes back to sleep.
pub(crate) fn watched_script(on: bool, keep_awake: bool) -> String {
    let (awake, interval_ms) = if on {
        (true, WATCHED_FRAME_INTERVAL_MS)
    } else {
        (keep_awake, HIDDEN_FRAME_INTERVAL_MS)
    };
    format!(
        "if (window.__idleManager) {{ window.__idleManager.setAwake({awake}); \
         window.__idleManager.setHiddenFrameInterval({interval_ms}); }}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shim_exposes_set_awake_and_set_hidden_frame_interval() {
        assert!(KEEP_AWAKE_JS.contains("window.__idleManager = {"));
        assert!(KEEP_AWAKE_JS.contains("setAwake:"));
        assert!(KEEP_AWAKE_JS.contains("setHiddenFrameInterval:"));
    }

    #[test]
    fn the_shim_tells_an_awake_page_it_is_visible() {
        assert!(
            KEEP_AWAKE_JS.contains("get() { return awake ? false : nativeHidden.get.call(this); }")
                && KEEP_AWAKE_JS.contains(
                    "get() { return awake ? 'visible' : nativeVisibilityState.get.call(this); }"
                )
                && KEEP_AWAKE_JS.contains("event.stopImmediatePropagation();")
        );
    }

    #[test]
    fn the_shim_intercepts_only_while_hidden_and_awake() {
        assert!(KEEP_AWAKE_JS.contains("const shimming = () => reallyHidden() && awake;"));
        assert!(KEEP_AWAKE_JS.contains("if (shimming()) {"));
    }

    #[test]
    fn the_shims_default_interval_equals_the_constant_set_watched_restores() {
        let declaration = format!("const HIDDEN_FRAME_INTERVAL_MS = {HIDDEN_FRAME_INTERVAL_MS};");

        assert!(
            KEEP_AWAKE_JS.contains(&declaration),
            "keep-awake.js must declare {declaration}"
        );
    }

    #[test]
    fn the_webkit_set_ends_with_the_prelude_only_for_a_keep_awake_account() {
        let awake = webkit_script_set(true);
        let plain = webkit_script_set(false);

        assert_eq!(awake.last(), Some(&KEEP_AWAKE_PRELUDE_JS));
        assert!(!plain.contains(&KEEP_AWAKE_PRELUDE_JS));
    }

    #[test]
    fn the_webkit_set_carries_the_shim_whether_or_not_keep_awake_is_on() {
        assert!(webkit_script_set(false).contains(&KEEP_AWAKE_JS));
        assert!(webkit_script_set(true).contains(&KEEP_AWAKE_JS));
    }

    #[test]
    fn the_shim_precedes_the_prelude_that_arms_it() {
        let scripts = webkit_script_set(true);

        let shim = scripts.iter().position(|s| *s == KEEP_AWAKE_JS);
        let prelude = scripts.iter().position(|s| *s == KEEP_AWAKE_PRELUDE_JS);
        assert!(shim < prelude);
    }

    #[test]
    fn the_webview2_set_ends_with_the_prelude_only_for_a_keep_awake_account() {
        let awake = webview2_script_set(true);
        let plain = webview2_script_set(false);

        assert_eq!(awake.last(), Some(&KEEP_AWAKE_PRELUDE_JS));
        assert!(!plain.contains(&KEEP_AWAKE_PRELUDE_JS));
    }

    #[test]
    fn the_webview2_set_is_the_bridge_followed_by_the_webkit_set() {
        for keep_awake in [false, true] {
            let mut expected = vec![WEBVIEW2_BRIDGE_JS];
            expected.extend(webkit_script_set(keep_awake));

            assert_eq!(webview2_script_set(keep_awake), expected);
        }
    }

    #[test]
    fn watching_arms_the_shim_at_the_watched_interval() {
        let script = watched_script(true, false);

        assert!(script.contains("setAwake(true)"));
        assert!(script.contains(&format!(
            "setHiddenFrameInterval({WATCHED_FRAME_INTERVAL_MS})"
        )));
    }

    #[test]
    fn leaving_restores_the_accounts_own_flag_and_the_hidden_interval() {
        let kept_awake = watched_script(false, true);
        let plain = watched_script(false, false);

        assert!(kept_awake.contains("setAwake(true)"));
        assert!(plain.contains("setAwake(false)"));
        assert!(plain.contains(&format!(
            "setHiddenFrameInterval({HIDDEN_FRAME_INTERVAL_MS})"
        )));
    }

    /// Runs the shim in a bare `JavaScriptCore` context — no display, no
    /// engine — behind a stand-in `window`/`document` that records whether a
    /// `requestAnimationFrame` reached the native call or a timer, then
    /// evaluates `scenario` and returns its result as a string.
    ///
    /// The stand-in `Document` carries `hidden` / `visibilityState` as
    /// prototype accessors over `setReallyHidden`, the way the engine's do,
    /// so the shim's spoof of both is what a scenario reads back; `dispatch`
    /// runs the window's capture listeners first with an event whose
    /// `stopImmediatePropagation` ends the run, as the real one does, and
    /// `pageSaw` counts what reached a listener the page registered.
    #[cfg(target_os = "linux")]
    fn run_shim(scenario: &str) -> String {
        const HARNESS_JS: &str = r"
            globalThis.window = globalThis;
            let reallyHidden = false;
            globalThis.setReallyHidden = (on) => { reallyHidden = on; };
            class Document {
              get hidden() { return reallyHidden; }
              get visibilityState() { return reallyHidden ? 'hidden' : 'visible'; }
              addEventListener(name, fn) { pageListeners.set(name, [...(pageListeners.get(name) || []), fn]); }
              dispatchEvent(event) { return dispatch(event.type); }
            }
            globalThis.Document = Document;
            globalThis.document = new Document();
            globalThis.Event = class { constructor(type) { this.type = type; this.stopped = false; } stopImmediatePropagation() { this.stopped = true; } };
            const captureListeners = new Map();
            const pageListeners = new Map();
            globalThis.addEventListener = (name, fn, capture) => {
              const map = capture ? captureListeners : pageListeners;
              map.set(name, [...(map.get(name) || []), fn]);
            };
            globalThis.pageSaw = [];
            globalThis.dispatch = (name) => {
              const event = new Event(name);
              for (const fn of captureListeners.get(name) || []) { fn(event); if (event.stopped) return false; }
              for (const fn of pageListeners.get(name) || []) { pageSaw.push(name); fn(event); }
              return true;
            };
            globalThis.nativeRequests = [];
            globalThis.nativeCancels = [];
            globalThis.timers = [];
            globalThis.requestAnimationFrame = (callback) => {
              nativeRequests.push(callback);
              return nativeRequests.length;
            };
            globalThis.cancelAnimationFrame = (id) => { nativeCancels.push(id); };
            globalThis.setTimeout = (callback, ms) => { timers.push({ callback, ms }); return timers.length; };
            globalThis.clearTimeout = () => {};
            globalThis.performance = { now: () => 0 };
        ";

        let context = webkit6::javascriptcore::Context::new();
        for (name, code) in [
            ("harness", HARNESS_JS),
            ("shim", KEEP_AWAKE_JS),
            ("scenario", scenario),
        ] {
            let value = context.evaluate(code);
            if let Some(exception) = context.exception() {
                panic!(
                    "{name} threw at line {}: {}",
                    exception.line_number(),
                    exception.message().unwrap_or_default()
                );
            }
            if name == "scenario" {
                return value
                    .map(|value| value.to_str().to_string())
                    .unwrap_or_default();
            }
        }
        unreachable!("the scenario always returns")
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_the_shim_exposes_both_runtime_calls() {
        let result = run_shim(
            "typeof window.__idleManager.setAwake + ' ' + typeof window.__idleManager.setHiddenFrameInterval",
        );

        assert_eq!(result, "function function");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_a_hidden_page_that_is_not_armed_keeps_the_native_request() {
        let result = run_shim(
            "setReallyHidden(true); requestAnimationFrame(() => {}); \
             JSON.stringify([nativeRequests.length, timers.length])",
        );

        assert_eq!(result, "[1,0]");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_a_hidden_armed_page_is_answered_from_a_timer() {
        let result = run_shim(
            "setReallyHidden(true); window.__idleManager.setAwake(true); \
             requestAnimationFrame(() => {}); \
             JSON.stringify([nativeRequests.length, timers.length])",
        );

        assert_eq!(result, "[0,1]");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_a_visible_armed_page_keeps_the_native_request() {
        let result = run_shim(
            "window.__idleManager.setAwake(true); requestAnimationFrame(() => {}); \
             JSON.stringify([nativeRequests.length, timers.length])",
        );

        assert_eq!(result, "[1,0]");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_the_timer_fires_at_the_interval_set_at_run_time() {
        let result = run_shim(&format!(
            "setReallyHidden(true); window.__idleManager.setAwake(true); \
             window.__idleManager.setHiddenFrameInterval({WATCHED_FRAME_INTERVAL_MS}); \
             requestAnimationFrame(() => {{}}); String(timers[0].ms)"
        ));

        assert_eq!(result, WATCHED_FRAME_INTERVAL_MS.to_string());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_arming_a_hidden_page_takes_over_the_request_the_engine_owes() {
        let result = run_shim(
            "setReallyHidden(true); const id = requestAnimationFrame(() => {}); \
             window.__idleManager.setAwake(true); \
             JSON.stringify([nativeCancels[0] === id, timers.length])",
        );

        assert_eq!(result, "[true,1]");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_an_armed_page_reads_itself_visible_while_really_hidden() {
        let result = run_shim(
            "setReallyHidden(true); window.__idleManager.setAwake(true); \
             JSON.stringify([document.hidden, document.visibilityState])",
        );

        assert_eq!(result, r#"[false,"visible"]"#);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_a_page_that_is_not_armed_reads_the_engines_own_answer() {
        let result = run_shim(
            "setReallyHidden(true); JSON.stringify([document.hidden, document.visibilityState])",
        );

        assert_eq!(result, r#"[true,"hidden"]"#);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_the_engines_visibilitychange_never_reaches_an_armed_page() {
        // Armed and then really hidden: the engine's event stops at the shim.
        // Disarmed while still hidden: the shim replays one, and the engine's
        // next one passes — two in all reach the page.
        let result = run_shim(
            "document.addEventListener('visibilitychange', () => {}); \
             window.__idleManager.setAwake(true); setReallyHidden(true); dispatch('visibilitychange'); \
             const whileArmed = pageSaw.length; \
             window.__idleManager.setAwake(false); dispatch('visibilitychange'); \
             JSON.stringify([whileArmed, pageSaw.length])",
        );

        assert_eq!(result, "[0,2]");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_arming_a_really_hidden_page_replays_one_visibilitychange_that_reads_visible() {
        let result = run_shim(
            "const seen = []; \
             document.addEventListener('visibilitychange', () => { seen.push(document.hidden); }); \
             setReallyHidden(true); dispatch('visibilitychange'); \
             window.__idleManager.setAwake(true); window.__idleManager.setAwake(true); \
             window.__idleManager.setAwake(false); \
             JSON.stringify(seen)",
        );

        assert_eq!(result, "[true,false,true]");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn evaluated_disarming_lets_a_hidden_page_fall_back_to_the_engine() {
        let result = run_shim(
            "setReallyHidden(true); window.__idleManager.setAwake(true); \
             window.__idleManager.setAwake(false); requestAnimationFrame(() => {}); \
             JSON.stringify([nativeRequests.length, timers.length])",
        );

        assert_eq!(result, "[1,0]");
    }
}
