// Forwards the page's own console output and its uncaught errors to the host,
// which logs them through `tracing`. WebKit's console-to-stdout stream carries
// the engine's messages — blocked frames, cancelled cross-origin loads — but as
// unstructured text with no origin attached, so it cannot be filtered when a
// login fails inside one noisy third-party frame among several.
//
// Injected into every frame at document start, before the page's own scripts
// run, so nothing the page logs during start-up is missed.
(() => {
  const handler =
    window.webkit &&
    window.webkit.messageHandlers &&
    window.webkit.messageHandlers.pageConsole;
  if (!handler) return;

  const MAX_TEXT = 2000;
  const LEVELS = ['debug', 'log', 'info', 'warn', 'error'];

  const render = (value) => {
    if (typeof value === 'string') return value;
    if (value instanceof Error) return `${value.name}: ${value.message}`;
    try {
      return JSON.stringify(value);
    } catch {
      return String(value);
    }
  };

  const send = (level, parts) => {
    try {
      handler.postMessage({
        level,
        origin: location.origin,
        text: parts.map(render).join(' ').slice(0, MAX_TEXT),
      });
    } catch {
      // A page must never break because logging it did.
    }
  };

  for (const level of LEVELS) {
    const original = console[level];
    console[level] = function (...parts) {
      send(level, parts);
      return original.apply(console, parts);
    };
  }

  addEventListener('error', (event) =>
    send('error', [event.message, `${event.filename}:${event.lineno}`]));
  addEventListener('unhandledrejection', (event) =>
    send('error', ['unhandled rejection:', event.reason]));
})();
