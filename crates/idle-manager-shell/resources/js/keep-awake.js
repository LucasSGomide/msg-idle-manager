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
//
// Since roadmap item 13 it is injected into *every* page and stays dormant
// until armed: it only answers from the timer while the document is hidden
// *and* `awake` is set. A keep-awake account's prelude arms it before the
// page runs, so item 04's behaviour is unchanged; the phone's wake arms it at
// run time through `window.__idleManager` without a reload, and disarms it
// again when the phone leaves (`FR.4.3`).
//
// Observers are the third thing a hidden page never gets. `ResizeObserver`
// and `IntersectionObserver` deliver inside the engine's rendering update,
// which does not run for a hidden page — measured 2026-09-20 (item 13 task
// 08): a page loaded while the window was minimised counted `ro=0 io=0`
// after twelve seconds — and a page that sizes its panels from a
// `ResizeObserver` or renders them on `IntersectionObserver` (Angular's
// `@defer (on viewport)`) then stays half-built until the window is next
// looked at. So while armed on a really hidden page the shim delivers both
// from a timer of its own: an observation whenever a target's box, or its
// intersection with the root, differs from the last one delivered, native
// or not. The engine's own delivery resumes untouched once the page is
// looked at again; what it delivers is recorded so the timer never repeats it.
//
// Frames alone were not enough. Measured on the owner's GNOME desktop
// (2026-09-20, item 13 task 08): with the window minimised, the phone kept
// receiving fresh pictures but the game in them stood still, because the
// page still read `document.hidden === true` and got its `visibilitychange`
// — and a Phaser game pauses its own loop on exactly that, as most games
// do. So while armed the page is also *told* it is visible: `hidden` and
// `visibilityState` answer as if the window were on screen, and
// `visibilitychange` never reaches the page's listeners. Arming or
// disarming while the page is really hidden replays one `visibilitychange`
// so a game that paused resumes, and one that was kept awake pauses again.
(() => {
  // A guess, not a measurement: FR.6.3 asks for the shim and names no rate.
  // Tried against the repository's own `vischeck` page (a counter driven only
  // by frame callbacks) and against Kittens Game, which played normally with
  // it installed. Too slow loses progress in a game that counts frames; too
  // fast spends processor time on every hidden account at once. Must equal
  // `HIDDEN_FRAME_INTERVAL_MS` in `web_view.rs`, which restores it after a
  // phone stops watching.
  const HIDDEN_FRAME_INTERVAL_MS = 250;

  // The engine hands out ids starting at 1 and counting up. Handing out ids
  // that count down from a large, disjoint range keeps a timer-sourced id
  // from ever colliding with one the engine issued, so `cancelAnimationFrame`
  // can tell which implementation owns a given id without tagging it.
  const FIRST_SHIM_FRAME_ID = Number.MAX_SAFE_INTEGER;

  const nativeRequestFrame = window.requestAnimationFrame.bind(window);
  const nativeCancelFrame = window.cancelAnimationFrame.bind(window);

  // The runtime flag. Off until the prelude or `setAwake(true)` turns it on;
  // with it off the shim is a pass-through and the engine keeps its own
  // hidden-page behaviour.
  let awake = false;
  let hiddenFrameIntervalMs = HIDDEN_FRAME_INTERVAL_MS;

  // What the engine really says, kept for the shim's own decisions — the
  // page sees the spoofed answer below.
  const nativeHidden = Object.getOwnPropertyDescriptor(Document.prototype, 'hidden');
  const nativeVisibilityState = Object.getOwnPropertyDescriptor(Document.prototype, 'visibilityState');
  const reallyHidden = () => nativeHidden.get.call(document);

  const shimming = () => reallyHidden() && awake;

  Object.defineProperty(Document.prototype, 'hidden', {
    configurable: true,
    enumerable: nativeHidden.enumerable,
    get() { return awake ? false : nativeHidden.get.call(this); },
  });
  Object.defineProperty(Document.prototype, 'visibilityState', {
    configurable: true,
    enumerable: nativeVisibilityState.enumerable,
    get() { return awake ? 'visible' : nativeVisibilityState.get.call(this); },
  });

  // The one `visibilitychange` the shim itself raises, so the page's
  // listeners see the arm or disarm as the change of state it is to them.
  let replaying = false;
  const replayVisibilityChange = () => {
    replaying = true;
    try {
      document.dispatchEvent(new Event('visibilitychange', { bubbles: true }));
    } finally {
      replaying = false;
    }
  };

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
    }, hiddenFrameIntervalMs);
    pendingTimers.set(frameId, timerId);
  };

  // Hand every request the engine is about to abandon (or has already
  // abandoned, when armed on a page that is hidden right now) over to the
  // timer, keeping the id the page already holds so its own
  // `cancelAnimationFrame` still reaches it.
  const takeOverPendingNative = () => {
    for (const [frameId, callback] of pendingNative) {
      nativeCancelFrame(frameId);
      pendingNative.delete(frameId);
      answerFromTimer(frameId, callback);
    }
  };

  window.requestAnimationFrame = (callback) => {
    if (shimming()) {
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

  // Capture phase on the window, registered before the page's own scripts
  // run, so it is first in line: while armed the engine's event stops here
  // and the page never learns it was hidden. The shim's own takeover happens
  // in the same place, since nothing after it would fire.
  addEventListener('visibilitychange', (event) => {
    if (shimming()) {
      takeOverPendingNative();
    }
    syncObserverTimer();
    if (awake && !replaying) {
      event.stopImmediatePropagation();
    }
  }, true);

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

  // --- observers ---------------------------------------------------------

  // How often the shim looks at every observed target while it stands in
  // for the engine. A box read forces layout, so this is slower than the
  // frame timer; a HUD settling within a quarter second is not noticed.
  const OBSERVER_POLL_INTERVAL_MS = 250;

  const shimObservers = new Set();
  let observerTimer = null;

  const boxKey = (target) => {
    const rect = target.getBoundingClientRect();
    return `${rect.width}x${rect.height}`;
  };

  const rectOf = (x, y, width, height) =>
    (typeof DOMRectReadOnly === 'function' ? DOMRectReadOnly.fromRect({ x, y, width, height }) : { x, y, width, height, top: y, left: x, right: x + width, bottom: y + height });

  const contentBox = (target) => {
    const rect = target.getBoundingClientRect();
    const style = getComputedStyle(target);
    const px = (value) => parseFloat(value) || 0;
    const width = Math.max(0, rect.width - px(style.paddingLeft) - px(style.paddingRight) - px(style.borderLeftWidth) - px(style.borderRightWidth));
    const height = Math.max(0, rect.height - px(style.paddingTop) - px(style.paddingBottom) - px(style.borderTopWidth) - px(style.borderBottomWidth));
    return { rect, width, height, padding: { left: px(style.paddingLeft), top: px(style.paddingTop) } };
  };

  const sizeEntry = (target) => {
    const box = contentBox(target);
    const dpr = window.devicePixelRatio || 1;
    return {
      target,
      contentRect: rectOf(box.padding.left, box.padding.top, box.width, box.height),
      borderBoxSize: [{ inlineSize: box.rect.width, blockSize: box.rect.height }],
      contentBoxSize: [{ inlineSize: box.width, blockSize: box.height }],
      devicePixelContentBoxSize: [{ inlineSize: Math.round(box.width * dpr), blockSize: Math.round(box.height * dpr) }],
    };
  };

  const rootRectOf = (root) => {
    if (root && typeof root.getBoundingClientRect === 'function') {
      return root.getBoundingClientRect();
    }
    return rectOf(0, 0, window.innerWidth, window.innerHeight);
  };

  const intersectionOf = (target, root) => {
    const bounds = target.getBoundingClientRect();
    const rootBounds = rootRectOf(root);
    const left = Math.max(bounds.left, rootBounds.left);
    const top = Math.max(bounds.top, rootBounds.top);
    const right = Math.min(bounds.right, rootBounds.right);
    const bottom = Math.min(bounds.bottom, rootBounds.bottom);
    const intersects = right >= left && bottom >= top;
    const width = intersects ? right - left : 0;
    const height = intersects ? bottom - top : 0;
    const area = bounds.width * bounds.height;
    const ratio = !intersects ? 0 : area === 0 ? 1 : (width * height) / area;
    return {
      target,
      isIntersecting: intersects,
      intersectionRatio: ratio,
      boundingClientRect: bounds,
      intersectionRect: rectOf(intersects ? left : 0, intersects ? top : 0, width, height),
      rootBounds,
      time: performance.now(),
    };
  };

  const intersectionKey = (entry, thresholds) => {
    let crossed = -1;
    for (let index = 0; index < thresholds.length; index += 1) {
      if (entry.intersectionRatio >= thresholds[index]) { crossed = index; }
    }
    return `${entry.isIntersecting}:${crossed}`;
  };

  const pollObservers = () => {
    if (!shimming()) { return; }
    for (const observer of shimObservers) {
      const entries = [];
      for (const [target, last] of observer.targets) {
        if (!target.isConnected) { continue; }
        const entry = observer.kind === 'resize' ? sizeEntry(target) : intersectionOf(target, observer.root);
        const key = observer.kind === 'resize' ? boxKey(target) : intersectionKey(entry, observer.thresholds);
        if (key !== last) {
          observer.targets.set(target, key);
          entries.push(entry);
        }
      }
      if (entries.length > 0) {
        try { observer.callback(entries, observer.facade); } catch (error) { setTimeout(() => { throw error; }); }
      }
    }
  };

  const syncObserverTimer = () => {
    const wanted = shimming() && shimObservers.size > 0;
    if (wanted && observerTimer === null) {
      observerTimer = setInterval(pollObservers, OBSERVER_POLL_INTERVAL_MS);
    } else if (!wanted && observerTimer !== null) {
      clearInterval(observerTimer);
      observerTimer = null;
    }
  };

  // Each facade holds a native observer that does the real work whenever
  // the engine delivers, and records what it delivered so the timer only
  // ever adds what the engine withheld.
  const wrapObserver = (kind, Native) => {
    if (typeof Native !== 'function') { return; }
    const facades = new WeakMap();
    class Facade {
      constructor(callback, options) {
        const record = { kind, callback, facade: this, targets: new Map(), root: null, thresholds: [0] };
        if (kind === 'intersection') {
          record.root = options && options.root ? options.root : null;
          const raw = options && options.threshold !== undefined ? options.threshold : [0];
          record.thresholds = (Array.isArray(raw) ? raw : [raw]).map(Number).sort((a, b) => a - b);
        }
        record.native = new Native((entries) => {
          for (const entry of entries) {
            if (record.targets.has(entry.target)) {
              record.targets.set(entry.target, kind === 'resize' ? boxKey(entry.target) : intersectionKey(entry, record.thresholds));
            }
          }
          callback(entries, this);
        }, options);
        facades.set(this, record);
        shimObservers.add(record);
        syncObserverTimer();
      }
      observe(target, options) {
        const record = facades.get(this);
        record.native.observe(target, options);
        if (!record.targets.has(target)) { record.targets.set(target, null); }
        syncObserverTimer();
      }
      unobserve(target) {
        const record = facades.get(this);
        record.native.unobserve(target);
        record.targets.delete(target);
      }
      disconnect() {
        const record = facades.get(this);
        record.native.disconnect();
        record.targets.clear();
        shimObservers.delete(record);
        syncObserverTimer();
      }
      takeRecords() { return facades.get(this).native.takeRecords(); }
      get root() { return facades.get(this).native.root; }
      get rootMargin() { return facades.get(this).native.rootMargin; }
      get thresholds() { return facades.get(this).native.thresholds; }
    }
    Object.defineProperty(Facade, 'name', { value: Native.name });
    return Facade;
  };

  const ShimResizeObserver = wrapObserver('resize', window.ResizeObserver);
  if (ShimResizeObserver) { window.ResizeObserver = ShimResizeObserver; }
  const ShimIntersectionObserver = wrapObserver('intersection', window.IntersectionObserver);
  if (ShimIntersectionObserver) { window.IntersectionObserver = ShimIntersectionObserver; }

  window.__idleManager = {
    setAwake: (on) => {
      const next = Boolean(on);
      const changed = next !== awake;
      awake = next;
      if (shimming()) {
        takeOverPendingNative();
      }
      syncObserverTimer();
      if (changed && reallyHidden()) {
        replayVisibilityChange();
      }
    },
    setHiddenFrameInterval: (ms) => {
      const interval = Number(ms);
      if (Number.isFinite(interval) && interval > 0) {
        hiddenFrameIntervalMs = interval;
      }
    },
  };
})();
