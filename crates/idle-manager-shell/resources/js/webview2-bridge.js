// Makes the WebKit message-handler shape `page-console.js` (and a later
// slice's `keep-awake.js`) already use work unchanged on WebView2, whose own
// bridge is a single `window.ipc.postMessage(string)` rather than one handler
// object per name. `window.ipc` is injected by wry itself; this only adds the
// `window.webkit.messageHandlers.<name>.postMessage(body)` shape on top of it
// and routes each call through as `{handler: name, body}`, stringified once
// here so the Rust side parses one JSON shape regardless of which handler
// posted it.
//
// Injected into every frame at document start, before any other script this
// crate injects, so `window.webkit` exists by the time they run.
// Also carries the `Ctrl`+wheel half of zoom (`FR.1.7`): WebView2's own
// `ICoreWebView2Controller` sees this page's DOM events, not GTK's, so the
// notch has to leave through the same IPC channel as everything else here,
// one message per event to match item 09's `DISCRETE` zoom behaviour. The
// listener is capture-phase so it runs even when the page's own script
// calls `stopPropagation`, and `preventDefault` on a matched event stops the
// page from also scrolling under the zoom gesture.
(() => {
  if (!window.ipc || !window.ipc.postMessage) return;

  const post = (handler, body) => window.ipc.postMessage(JSON.stringify({ handler, body }));

  const handlerNames = ['pageConsole'];
  window.webkit = window.webkit || {};
  window.webkit.messageHandlers = window.webkit.messageHandlers || {};
  for (const name of handlerNames) {
    window.webkit.messageHandlers[name] = {
      postMessage: (body) => post(name, body),
    };
  }

  window.addEventListener(
    'wheel',
    (event) => {
      if (!event.ctrlKey || event.deltaY === 0) return;
      event.preventDefault();
      post('zoomStep', Math.sign(event.deltaY));
    },
    { capture: true, passive: false },
  );
})();
