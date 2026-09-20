#!/usr/bin/env node
// A scripted phone that measures roadmap item 13's two figures: frames per
// second while attached, and tap-to-visible latency — the time from sending a
// tap to the first frame that shows its effect. It enrols and attaches the
// way scripts/phone-probe.mjs does, then taps the page `page` writes: each
// tap toggles a full-screen noise picture, so a frame's JPEG size says which
// side of the toggle it shows and no image decoder is needed. Node 22+.
//
//   node scripts/phone-latency.mjs page <file>
//       writes the tap page to <file>; point one account's url at file://<file>
//   node scripts/phone-latency.mjs <enrolment-url> [watch-secs] [taps]
//       counts frames for watch-secs (default 10), then taps `taps` times
//       (default 10) and prints each latency and the median
//
// <enrolment-url> is the `debug switch: enrolment offered address=…` line the
// desktop logs under IDLE_MANAGER_DEBUG_ENROL=1, or the address the phone
// dialog shows.

import { createHmac, randomBytes } from "node:crypto";
import { writeFileSync } from "node:fs";

const PAGE = `<!doctype html>
<meta name="viewport" content="width=device-width">
<style>
  html, body { margin: 0; height: 100%; background: #202030; color: #ddd; font: 24px monospace; overflow: hidden; }
  #clock { position: fixed; top: 24px; left: 24px; }
  #noise { position: fixed; inset: 0; width: 100%; height: 100%; display: none; }
  body.noisy #noise { display: block; }
</style>
<div id="clock">0</div>
<canvas id="noise"></canvas>
<script>
  // A ticking clock so every frame differs, as a game's would.
  const clock = document.getElementById("clock");
  let ticks = 0;
  setInterval(() => { clock.textContent = String(++ticks); }, 100);
  // The noise picture, drawn once: random pixels compress to a JPEG many times
  // the size of the plain page, which is what the probe keys on.
  const canvas = document.getElementById("noise");
  canvas.width = window.innerWidth; canvas.height = window.innerHeight;
  const context = canvas.getContext("2d");
  const image = context.createImageData(canvas.width, canvas.height);
  for (let i = 0; i < image.data.length; i++) image.data[i] = (i % 4 === 3) ? 255 : Math.random() * 256;
  context.putImageData(image, 0, 0);
  document.body.addEventListener("click", () => document.body.classList.toggle("noisy"));
</script>`;

const [first, second, third] = process.argv.slice(2);
if (first === "page") {
  if (!second) { console.error("usage: phone-latency.mjs page <file>"); process.exit(2); }
  writeFileSync(second, PAGE);
  console.log(`wrote ${second}`);
  process.exit(0);
}
if (!first) {
  console.error("usage: phone-latency.mjs <enrolment-url> [watch-secs] [taps] | page <file>");
  process.exit(2);
}
const enrolUrl = first;
const watchSecs = Number(second ?? 10);
const tapCount = Number(third ?? 10);
const COOKIE = "idle-manager-phone";
const VIEWPORT = { width: 412, height: 915 };
const PING_INTERVAL_MS = 5000;
const TAP_POINT = { x: 200, y: 500 };
const TAP_TIMEOUT_MS = 5000;

const hmacHex = (secret, text) => createHmac("sha256", secret).update(text).digest("hex");
const meta = (page, name) => page.match(new RegExp(`<meta name="${name}" content="([0-9a-f]*)"`))?.[1];
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const median = (values) => { const sorted = [...values].sort((a, b) => a - b); return sorted[Math.floor(sorted.length / 2)]; };

// --- enrol and open the socket, as phone-probe.mjs does ------------------------
const response = await fetch(enrolUrl);
if (response.status !== 200) throw new Error(`enrolment answered ${response.status}`);
const page = await response.text();
const deviceId = response.headers.get("set-cookie")?.match(new RegExp(`${COOKIE}=([0-9a-f]+)`))?.[1];
const secret = Buffer.from(meta(page, "idle-manager-secret"), "hex");
if (!deviceId || secret.length !== 32) throw new Error("the enrolment page carried no credential");
console.log(`enrolled device=${deviceId}`);

const host = new URL(enrolUrl).host;
const ws = new WebSocket(`ws://${host}/ws`, { headers: { Cookie: `${COOKIE}=${deviceId}` } });
ws.binaryType = "arraybuffer";
const texts = [];
const frames = [];
let waiter = null;
ws.onmessage = (event) => {
  if (typeof event.data === "string") {
    texts.push(JSON.parse(event.data));
  } else {
    const bytes = Buffer.from(event.data);
    frames.push({ at: performance.now(), size: bytes.length - 8 });
  }
  waiter?.();
};
ws.onclose = (event) => console.log(`socket closed code=${event.code}`);
await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = () => reject(new Error("socket failed")); });

const send = (message) => ws.send(JSON.stringify(message));
async function nextText(matches, timeoutMs = 5000) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const index = texts.findIndex(matches);
    if (index >= 0) return texts.splice(index, 1)[0];
    if (Date.now() > deadline) throw new Error("timed out waiting for a message");
    await new Promise((resolve) => { waiter = resolve; setTimeout(resolve, 50); });
  }
}

const hello = await nextText((message) => message.type === "hello");
const phoneChallenge = randomBytes(32).toString("hex");
send({ type: "auth", proof: hmacHex(secret, `phone|${hello.challenge}`), challenge: phoneChallenge });
const welcome = await nextText((message) => message.type === "welcome");
if (welcome.proof !== hmacHex(secret, `desktop|${phoneChallenge}`)) throw new Error("the desktop's proof did not verify");
const pinger = setInterval(() => send({ type: "ping" }), PING_INTERVAL_MS);

send({ type: "mobile", on: true });
await nextText((message) => message.type === "state" && message.state.mobileMode === true);
send({ type: "attach", viewport: VIEWPORT });
const attached = await nextText((message) => message.type === "state" && message.state.viewport.width === VIEWPORT.width);
console.log(`attached current=${attached.state.current}`);

// --- frames per second ---------------------------------------------------------
const from = frames.length;
const started = performance.now();
await sleep(watchSecs * 1000);
const watched = frames.slice(from);
const elapsed = (performance.now() - started) / 1000;
console.log(`frames in ${elapsed.toFixed(1)}s: ${watched.length} (${(watched.length / elapsed).toFixed(1)}/s), median ${median(watched.map((frame) => frame.size))} bytes`);

// --- tap-to-visible latency -------------------------------------------------------
// The plain page and the noisy page sit far apart in JPEG size; the threshold
// is learnt from the first tap, whose frames straddle both.
const plainSize = median(watched.map((frame) => frame.size));
let threshold = null;
const latencies = [];
for (let tap = 0; tap < tapCount; tap++) {
  const before = frames.length;
  const noisyExpected = tap % 2 === 0;
  const sentAt = performance.now();
  send({ type: "tap", ...TAP_POINT });
  let seenAt = null;
  while (performance.now() - sentAt < TAP_TIMEOUT_MS) {
    const fresh = frames.slice(before);
    if (threshold === null) {
      const big = fresh.find((frame) => frame.size > plainSize * 3);
      if (big) threshold = (plainSize + big.size) / 2;
    }
    if (threshold !== null) {
      const flipped = fresh.find((frame) => (frame.size > threshold) === noisyExpected);
      if (flipped) { seenAt = flipped.at; break; }
    }
    await new Promise((resolve) => { waiter = resolve; setTimeout(resolve, 5); });
  }
  if (seenAt === null) { console.log(`tap ${tap + 1}: no visible effect within ${TAP_TIMEOUT_MS} ms`); continue; }
  latencies.push(seenAt - sentAt);
  console.log(`tap ${tap + 1}: ${(seenAt - sentAt).toFixed(0)} ms (${noisyExpected ? "to noise" : "to plain"})`);
  await sleep(1500);
}
if (latencies.length) {
  console.log(`tap-to-visible: median ${median(latencies).toFixed(0)} ms over ${latencies.length} taps (min ${Math.min(...latencies).toFixed(0)}, max ${Math.max(...latencies).toFixed(0)}), threshold ${threshold?.toFixed(0)} bytes`);
}

send({ type: "leave" });
clearInterval(pinger);
ws.close();
await sleep(200);
process.exit(0);
