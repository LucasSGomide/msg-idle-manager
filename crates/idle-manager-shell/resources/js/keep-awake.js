// WebKit stops handing a hidden page its animation frame callback at all —
// zero calls, not a slower rate — with no setting anywhere to prevent it
// (task 02 measured 22 seconds minimised without a single callback). This
// shim is the only way past that: while the document reports itself hidden,
// answer `requestAnimationFrame` from a timer instead of the engine, so a
// game whose whole loop is driven by that callback keeps running rather than
// stopping until the page is looked at again.
//
// Injected at document start, before the page's own scripts run, so the
// replacement is already in place by the time a game calls
// `requestAnimationFrame` for the first time.
(() => {
  // A guess, not a measurement: FR.6.3 asks for the shim and names no rate.
  // Tried against the repository's own `vischeck` page (a counter driven only
  // by frame callbacks) and against Kittens Game, which played normally with
  // it installed. Too slow loses progress in a game that counts frames; too
  // fast spends processor time on every hidden account at once.
  const HIDDEN_FRAME_INTERVAL_MS = 250;

  // The engine hands out ids starting at 1 and counting up. Handing out ids
  // that count down from a large, disjoint range keeps a timer-sourced id
  // from ever colliding with one the engine issued, so `cancelAnimationFrame`
  // can tell which implementation owns a given id without tagging it.
  const FIRST_SHIM_FRAME_ID = Number.MAX_SAFE_INTEGER;

  const nativeRequestFrame = window.requestAnimationFrame.bind(window);
  const nativeCancelFrame = window.cancelAnimationFrame.bind(window);

  // Timer id, keyed by the shim's frame id, for a pending hidden-page
  // request that has not fired yet. Lets a cancel reach a timer that native
  // `cancelAnimationFrame` has never heard of.
  const pendingTimers = new Map();

  // Callbacks the engine still owes us, keyed by the id it issued. A game's
  // loop always has exactly one request outstanding at the moment the page
  // hides, and the engine never delivers that one — so intercepting only new
  // calls would leave the loop dead on the very request that would have
  // restarted it.
  const pendingNative = new Map();
  let nextShimFrameId = FIRST_SHIM_FRAME_ID;

  const answerFromTimer = (frameId, callback) => {
    const timerId = setTimeout(() => {
      pendingTimers.delete(frameId);
      callback(performance.now());
    }, HIDDEN_FRAME_INTERVAL_MS);
    pendingTimers.set(frameId, timerId);
  };

  window.requestAnimationFrame = (callback) => {
    if (document.hidden) {
      const frameId = nextShimFrameId--;
      answerFromTimer(frameId, callback);
      return frameId;
    }

    const frameId = nativeRequestFrame((timestamp) => {
      pendingNative.delete(frameId);
      callback(timestamp);
    });
    pendingNative.set(frameId, callback);
    return frameId;
  };

  // Hand every request the engine is about to abandon over to the timer,
  // keeping the id the page already holds so its own `cancelAnimationFrame`
  // still reaches it.
  addEventListener('visibilitychange', () => {
    if (!document.hidden) {
      return;
    }
    for (const [frameId, callback] of pendingNative) {
      nativeCancelFrame(frameId);
      pendingNative.delete(frameId);
      answerFromTimer(frameId, callback);
    }
  });

  window.cancelAnimationFrame = (frameId) => {
    const timerId = pendingTimers.get(frameId);
    if (timerId === undefined) {
      pendingNative.delete(frameId);
      nativeCancelFrame(frameId);
      return;
    }
    pendingTimers.delete(frameId);
    clearTimeout(timerId);
  };
})();
