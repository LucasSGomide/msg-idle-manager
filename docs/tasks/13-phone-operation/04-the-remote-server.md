# 04 — The remote server: enrolment, the socket and its proof

**Roadmap:** [13](../../roadmap/13-phone-operation/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

A phone will watch and tap a game running on this always-on desktop. The
desktop needs a small server for that: something that listens on the private
mesh network the owner installs on both devices, lets exactly one phone in,
and exchanges a handful of messages with it. This slice builds that server
as its own crate, on its own thread, with no window and no web engine
involved, so it can be tested end to end with a plain network client.

The server serves three addresses and nothing else. One consumes a one-time
enrolment code the desktop shows as a QR picture; it mints a secret for the
phone, remembers the phone as the only one allowed, and hands back the phone
page with that secret inside. One serves the same page again later, but only
to a request carrying the enrolled phone's cookie. One upgrades to a
persistent connection over which pictures flow one way and messages the
other. Every other request, and every request from a device that is not the
enrolled phone, gets an empty not-found answer with no hint of what lives
here.

On the persistent connection the server first demands proof. It sends a
random challenge; the phone must answer with a code computed from the secret;
the server answers with its own code so the phone can check the desktop too.
Only then does anything else pass. After that the server forwards the phone's
messages, attach, leave, choose, park, start, mobile, tap and scroll, onto a
channel the desktop window reads, and pushes the desktop's state and pictures
back down. Pictures arrive from the desktop as raw pixels on Linux and are
compressed here, on the server's thread, never on the window's. A phone that
falls silent for fifteen seconds is treated as gone. Un-enrolling from the
desktop clears the record, says goodbye and closes the connection at once.

The server has no user interface of its own. The desktop wiring and the phone
page that speak to it are separate slices, which is why this one can run
beside the desktop's mobile layout work.

## Technical details

- **Architecture** — `crates/idle-manager-remote/src/address.rs`:
  `bind_address(override: Option<&str>) -> Result<SocketAddr, StartError>`
  returns the override parsed, else the first IPv4 address in `100.64.0.0/10`
  from `if_addrs::get_if_addrs()` on `DEFAULT_PHONE_PORT = 7466`, else
  `StartError::NoMeshAddress` (code standards rules 5, 12).
- **Architecture** — `lib.rs`: `RemoteServer::start(config: RemoteConfig { bind }, record: Arc<dyn PhoneRecordStore>) -> Result<(RemoteHandle, async_channel::Receiver<RemoteIntent>), StartError>`
  binds a `TcpListener`, spawns an accept thread that hands each connection to
  a connection thread, all plain `std::thread`s over blocking `std::net`
  sockets — no `tokio` and no glib (`docs/stack.md`); `RemoteHandle: Clone + Send + Sync`
  over `Arc<Mutex<Shared>>` implements `PhoneLink` (rules 2, 3, 5): `begin_enrolment`
  mints a 32-byte hex code with `ENROLMENT_CODE_TTL_SECS = 600` and returns
  `EnrolmentOffer { address: "http://<bind>/enrol/<code>", expires_in_secs }`;
  `cancel_enrolment` drops it; `phone_status` reads the record and the
  attached flag; `publish_state` sends `state { … }` to the authenticated
  socket and keeps the last state for the next `welcome`; `revoke_phone`
  clears the record, sends `bye {reason: revoked}`, closes the socket and
  emits `RemoteIntent::Leave`.
- **Architecture** — `http.rs` parses one HTTP/1.1 request (method, path,
  headers) and answers exactly `GET /enrol/{code}`, `GET /` and `GET /ws` per
  `openapi.json`: enrolment consumes a live code once, mints a device id and
  a 32-byte secret with `rand`, writes `EnrolledPhone` (replacing any
  earlier), answers `200 text/html` with `phone.html` carrying the
  `<meta name="idle-manager-device-id">` and `<meta name="idle-manager-secret">`
  tags and `Set-Cookie: idle-manager-phone=<id>; Path=/; Max-Age=31536000; SameSite=Strict; HttpOnly`;
  `/` and `/ws` require that cookie to match; everything else and any failure
  is `404` with an empty body and no `Server` header. Until the phone page
  slice lands, `phone.html` is a placeholder file with the two meta tags and
  the title `Idle Manager`.
- **Architecture** — `websocket.rs`: the RFC 6455 handshake
  (`Sec-WebSocket-Accept` = base64(SHA-1(key + `258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)))
  and framing/unframing of text, binary, ping/pong and close frames with
  client masking, on blocking `TcpStream`s with a read timeout so the
  heartbeat check runs.
- **Architecture** — `session.rs`: sends `hello {challenge}`; verifies
  `auth.proof` as HMAC-SHA256(secret, "phone|" + challenge) in constant time
  with `hmac::Mac::verify_slice`, closing silently on failure; answers
  `welcome {proof: HMAC(secret, "desktop|" + phone challenge), state}`; a second
  authenticated socket sends `bye {reason: replaced}` to the first; forwards
  `attach`, `leave`, `choose`, `park`, `start`, `mobile`, `tap`, `scroll` as
  `RemoteIntent`s on the channel; drops any other type with `tracing::warn!`
  and no reply; feeds `ping` and `attach` into `Presence::observe(now_millis)`
  and emits `Leave` once when it reports gone (rules 8, 9, 10).
- **Architecture** — `protocol.rs`: serde wire types for every `Server*` and
  `Phone*` schema in `openapi.json`, `From<&RemoteState>` and
  `TryFrom<PhoneMessage> for RemoteIntent` (rule 7 applied to the wire); the
  `Liveness` word map is `live`, `parked`, `starting`, `queued`.
- **Architecture** — `frames.rs`: `publish_frame` places the `Frame` in a
  one-slot mailbox (latest wins); the connection thread encodes `Frame::Rgba`
  with `jpeg-encoder` at quality 75, passes `Frame::Jpeg` through, skips a
  frame whose bytes hash equals the last sent, and sends a binary message of
  big-endian `u32` width and height followed by the JPEG; the server never
  sends a frame while the phone is not attached or while the last published
  state has `mobile_mode` false (`FR.3.3`, `FR.4.2`), and encoding happens on
  the connection thread, never on the GTK main context (rule 10).
- **Code standards** — every failure is a `thiserror` enum or a `tracing`
  line with fields (rules 12, 15); no `unwrap` (rule 13); integration tests
  under `tests/` drive a real `TcpStream` and need no display (rules 24, 25;
  architecture rule 14).

## Acceptance criteria

- [x] `(unit)` `bind_address` prefers the override, else the first
      `100.64.0.0/10` address on port 7466, else `NoMeshAddress`
- [x] `(unit)` the handshake computes the RFC 6455 example accept value
      `s3pPLMBiTxaQ9kYGzzhZRbK+xOo=` for key `dGhlIHNhbXBsZSBub25jZQ==`
- [x] `(unit)` `TryFrom<PhoneMessage>` maps every `Phone*` variant to its
      `RemoteIntent` and rejects an unknown `type` with an error carrying the
      type name
- [x] `(integration)` `GET /enrol/<live code>` answers 200 with both meta
      tags and the `Set-Cookie` header, writes the record, and the same code
      a second time answers 404 with an empty body
- [x] `(integration)` `GET /`, `GET /ws` and `GET /anything` without the cookie,
      or with a mismatched id, answer 404 with an empty body and no `Server`
      header; `GET /` with the right cookie answers 200 with the page and no
      secret in it
- [x] `(integration)` on `/ws` a wrong `auth.proof` closes the socket with no
      message; a right one receives `welcome` whose `proof` verifies against
      the phone's challenge and whose `state` equals the last published; a
      second right one receives `welcome` while the first receives
      `bye {reason: replaced}` and is closed
- [x] `(integration)` after `welcome`, sending `attach`, `tap {x:1,y:2}` and
      `park {account}` yields the three matching `RemoteIntent`s on the channel
      in order, an unknown `type` yields nothing, and one `publish_state`
      arrives as exactly one `state` message
- [x] `(integration)` `publish_frame(Rgba)` is received as one binary message
      with the width/height header and a decodable JPEG; publishing the same
      pixels again sends nothing; frames published before `attach` send nothing
- [x] `(integration)` 15 s without `ping` (clock driven by the test) yields
      one `Leave`; `revoke_phone()` sends `bye {reason: revoked}`, closes the
      socket, yields `Leave`, and a following `phone_status()` is `NotEnrolled`
- [ ] `(integration)` `make verify` and `make windows-check` pass

## References

- [Roadmap item](../../roadmap/13-phone-operation/README.md) — Back-end
  "A sixth crate" (threads, no `tokio`), "The server", "The session";
  Technical References: the mesh network assigns `100.64.0.0/10` and
  encrypts the path; the RFC 6455 handshake is one SHA-1 plus base64 and the
  framing a 2–14 byte header with client masking, small enough to write
  against the RFC; `jpeg-encoder` encodes a 412 × 915 frame in single-digit
  milliseconds at quality 75
- [`openapi.json`](../../roadmap/13-phone-operation/openapi.json) — the
  three routes and every message schema
- [Sequence diagrams](../../roadmap/13-phone-operation/sequence-diagrams.md)
  — all three diagrams
- [Wireframes](../../roadmap/13-phone-operation/wireframes/) — the whole item
- [`docs/requirements.md`](../../requirements.md) — Remote Access `FR.4.4`,
  `FR.4.5`, `FR.5.1`, `FR.5.2`, `FR.6.1`, `FR.6.2`, `FR.6.3`
- [`docs/architecture.md`](../../architecture.md) — rules 2, 3, 5, 7, 8, 9,
  10, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 12, 13, 15,
  24, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 3, 10

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
