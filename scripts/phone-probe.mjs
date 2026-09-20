#!/usr/bin/env node
// A scripted phone for roadmap item 13's runbook: enrols against a running
// desktop, opens the socket, proves the secret, turns mobile mode on,
// attaches, counts and saves frames, taps, switches account, parks it,
// switches back and leaves — printing one line per step so a hand-run test
// has something to read against test-script.md. Node 22+ only, nothing to install: the global
// WebSocket carries the cookie header and `node:crypto` does the HMAC.
//
//   node scripts/phone-probe.mjs <enrolment-url> <out-dir> [watch-secs]
//
// <enrolment-url> is the `debug switch: enrolment offered address=…` line the
// desktop logs under IDLE_MANAGER_DEBUG_ENROL=1; <out-dir> receives
// frame-first.jpg, frame-late.jpg and frame-last.jpg; [watch-secs] is how
// long the first frame count runs (default 10).

import { createHmac, randomBytes } from "node:crypto";
import { writeFileSync } from "node:fs";
import { join } from "node:path";

const [enrolUrl, outDir, watchSecsArg] = process.argv.slice(2);
if (!enrolUrl || !outDir) {
  console.error("usage: phone-probe.mjs <enrolment-url> <out-dir> [watch-secs]");
  process.exit(2);
}
const watchSecs = Number(watchSecsArg ?? 10);
const COOKIE = "idle-manager-phone";
const VIEWPORT = { width: 412, height: 915 };
const PING_INTERVAL_MS = 5000;

const hmacHex = (secret, text) => createHmac("sha256", secret).update(text).digest("hex");
const meta = (page, name) => page.match(new RegExp(`<meta name="${name}" content="([0-9a-f]*)"`))?.[1];
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

// --- enrol -----------------------------------------------------------------
const response = await fetch(enrolUrl);
if (response.status !== 200) throw new Error(`enrolment answered ${response.status}`);
const page = await response.text();
const deviceId = response.headers.get("set-cookie")?.match(new RegExp(`${COOKIE}=([0-9a-f]+)`))?.[1];
const secret = Buffer.from(meta(page, "idle-manager-secret"), "hex");
if (!deviceId || secret.length !== 32) throw new Error("the enrolment page carried no credential");
console.log(`enrolled device=${deviceId} secret=${secret.length} bytes`);

// --- socket ----------------------------------------------------------------
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
    frames.push({ at: Date.now(), width: bytes.readUInt32BE(0), height: bytes.readUInt32BE(4), jpeg: bytes.subarray(8) });
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
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${matches}`);
    await new Promise((resolve) => { waiter = resolve; setTimeout(resolve, 100); });
  }
}
const ofType = (type) => (message) => message.type === type;

// --- auth ------------------------------------------------------------------
const hello = await nextText(ofType("hello"));
const phoneChallenge = randomBytes(32).toString("hex");
send({ type: "auth", proof: hmacHex(secret, `phone|${hello.challenge}`), challenge: phoneChallenge });
const welcome = await nextText(ofType("welcome"));
const proofOk = welcome.proof === hmacHex(secret, `desktop|${phoneChallenge}`);
console.log(`welcome proof=${proofOk ? "verified" : "WRONG"} mobileMode=${welcome.state.mobileMode} current=${welcome.state.current}`);
if (!proofOk) throw new Error("the desktop's proof did not verify");

const pinger = setInterval(() => send({ type: "ping" }), PING_INTERVAL_MS);

// --- mobile mode and attach --------------------------------------------------
send({ type: "mobile", on: true });
const mobileState = await nextText((message) => message.type === "state" && message.state.mobileMode === true);
console.log(`state after mobile: mobileMode=${mobileState.state.mobileMode} viewport=${mobileState.state.viewport.width}x${mobileState.state.viewport.height} current=${mobileState.state.current}`);
const accounts = mobileState.state.workspaces.flatMap((workspace) => workspace.accounts.map((account) => ({ ...account, workspace: workspace.name })));
console.log(`accounts: ${accounts.map((account) => `${account.id}(${account.workspace},${account.liveness})`).join(" ")}`);

send({ type: "attach", viewport: VIEWPORT });
const attachedState = await nextText((message) => message.type === "state" && message.state.viewport.width === VIEWPORT.width);
console.log(`state after attach: viewport=${attachedState.state.viewport.width}x${attachedState.state.viewport.height}`);

// --- count frames ----------------------------------------------------------
async function countFrames(seconds) {
  const from = frames.length;
  const started = Date.now();
  await sleep(seconds * 1000);
  const received = frames.slice(from);
  const distinct = received.filter((frame, index) => index === 0 || !frame.jpeg.equals(received[index - 1].jpeg)).length;
  const size = (frame) => (frame ? `${frame.width}x${frame.height}` : "none");
  console.log(`frames in ${seconds}s: ${received.length} (${(received.length / ((Date.now() - started) / 1000)).toFixed(1)}/s), ${distinct} distinct, first ${size(received[0])} last ${size(received.at(-1))}`);
  return received;
}
const first = await countFrames(watchSecs);
if (first.length) {
  const late = first.reduce((best, frame) => (frame.at <= first.at(-1).at - 5000 ? frame : best), first[0]);
  writeFileSync(join(outDir, "frame-first.jpg"), first[0].jpeg);
  writeFileSync(join(outDir, "frame-late.jpg"), late.jpeg);
  writeFileSync(join(outDir, "frame-last.jpg"), first.at(-1).jpeg);
  console.log(`saved first/late/last: first!=last ${!first[0].jpeg.equals(first.at(-1).jpeg)}, late!=last ${!late.jpeg.equals(first.at(-1).jpeg)} (late is ${((first.at(-1).at - late.at) / 1000).toFixed(1)}s before last)`);
}

// --- tap and scroll ------------------------------------------------------------
send({ type: "tap", x: 200, y: 150 });
send({ type: "scroll", x: 200, y: 400, dx: 0, dy: -40 });
console.log("sent tap (200,150) and scroll dy=-40");
await sleep(500);

// --- switch account, park it -----------------------------------------------------
const other = accounts.find((account) => account.id !== attachedState.state.current);
if (other) {
  send({ type: "choose", account: other.id });
  const chosen = await nextText((message) => message.type === "state" && message.state.current === other.id);
  console.log(`state after choose: current=${chosen.state.current} (workspace ${other.workspace})`);
  await countFrames(5);
  send({ type: "park", account: other.id });
  const parked = await nextText((message) => message.type === "state" && message.state.workspaces.some((workspace) => workspace.accounts.some((account) => account.id === other.id && account.liveness === "parked")));
  console.log(`state after park: ${other.id} liveness=parked, current=${parked.state.current}`);
  await countFrames(2);
  send({ type: "choose", account: attachedState.state.current });
  const back = await nextText((message) => message.type === "state" && message.state.current === attachedState.state.current);
  console.log(`state after choosing back: current=${back.state.current}`);
  await countFrames(2);
}

// --- leave -------------------------------------------------------------------
send({ type: "leave" });
console.log("sent leave");
await countFrames(3);
clearInterval(pinger);
ws.close();
await sleep(200);
process.exit(0);
