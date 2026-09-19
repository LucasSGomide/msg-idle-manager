# Sequence diagrams

All three routes are new: the application serves no HTTP today. They are
served by `idle-manager-remote`, the sixth crate, on its own thread; the rule
doc is `docs/architecture.md`.

**Endpoint:** `GET /enrol/{code}`

```mermaid
sequenceDiagram
    participant Dialog as PhoneDialog (GTK main context)
    participant Handle as RemoteHandle (PhoneLink)
    participant Http as http.rs (connection thread)
    participant Record as TomlPhoneRecord (PhoneRecordStore)
    participant Browser as Phone browser

    Dialog->>Handle: begin_enrolment()
    Handle->>Handle: mint code (32 random bytes, hex), expires in 600 s
    Handle-->>Dialog: EnrolmentOffer {address, expires_in_secs}
    Browser->>Http: GET /enrol/{code}
    Http->>Handle: consume_code(code, now)
    alt code unknown, already used or expired
        Handle-->>Http: None
        Http-->>Browser: 404, empty body, no Server header
    else code live
        Handle->>Handle: mint device id + 32-byte secret
        Handle->>Record: write(EnrolledPhone {device_id, secret, enrolled_on})
        alt phone.toml cannot be written
            Record-->>Handle: PhoneRecordError
            Handle-->>Http: error, code left unconsumed
            Http-->>Browser: 404, empty body
            Handle->>Handle: tracing::error! with the reason
        else written, replacing any earlier phone
            Record-->>Handle: Ok
            Handle-->>Http: credential
            Http-->>Browser: 200 text/html phone.html + meta tags, Set-Cookie idle-manager-phone=<device id>
            Dialog->>Handle: phone_status() (polled once a second)
            Handle-->>Dialog: Enrolled {attached: false}
        end
    end
```

**Architecture rules**

- Rule 2 — the remote adapter reaches the store only through the core's
  `PhoneRecordStore` port; the two adapters never depend on each other
- Rule 3 — the binary alone knows the record is a TOML file and hands the
  store into `RemoteServer::start`
- Rule 5 — `PhoneLink` and `PhoneRecordStore` are defined in the core for the
  capability, implemented outside for the technology
- Rule 7 — `phone.toml` has its own serde record types mapped from
  `EnrolledPhone`
- Rule 9 — code expiry takes `now` as an argument; the policy is tested with
  numbers
- Rule 10 — the dialog polls `phone_status()` from the GTK main context; the
  connection thread never touches a widget
- Rule 11 — `PhoneRecordError` is a `thiserror` enum defined beside the port in the core and returned by the store adapter

**Endpoint:** `GET /`

```mermaid
sequenceDiagram
    participant Browser as Phone browser
    participant Http as http.rs (connection thread)
    participant Handle as RemoteHandle (shared state)

    Browser->>Http: GET / with Cookie idle-manager-phone=<device id>
    Http->>Handle: enrolled_device_id()
    alt no phone enrolled, cookie missing or id mismatched
        Handle-->>Http: no match
        Http-->>Browser: 404, empty body, no Server header
    else match
        Handle-->>Http: match
        Http-->>Browser: 200 text/html phone.html (no credential embedded)
    end
```

**Architecture rules**

- Rule 8 — serving the page changes no state; the page later drives the
  application only through intents
- Rule 14 — the route is covered by an integration test driving a plain TCP
  client, needing no display

**Endpoint:** `GET /ws`

```mermaid
sequenceDiagram
    participant Page as Phone page
    participant Http as http.rs → websocket.rs
    participant Session as session.rs (connection thread)
    participant Channel as async_channel<RemoteIntent>
    participant Window as Window (GTK main context)
    participant Record as TomlPhoneRecord

    Page->>Http: GET /ws, Upgrade: websocket, Cookie idle-manager-phone=<id>
    alt cookie missing or mismatched
        Http-->>Page: 404, empty body
    else
        Http-->>Page: 101, Sec-WebSocket-Accept
        Session-->>Page: hello {challenge}
        Page->>Session: auth {proof, challenge}
        Session->>Record: read() → secret
        alt proof wrong (constant-time compare)
            Session-->>Page: close, no message
        else proof right
            Session->>Session: replace any earlier authenticated socket (bye {reason: replaced})
            Session-->>Page: welcome {proof over "desktop|" + phone challenge, state}
            Page->>Session: attach {viewport}
            Session->>Channel: RemoteIntent::Attach {viewport}
            Channel->>Window: apply_remote_intent (spawn_future_local loop)
            loop while attached and mobile mode is on
                Window->>Session: publish_frame(Frame) → one-slot mailbox
                Session->>Session: encode JPEG q75, skip if hash unchanged
                Session-->>Page: binary frame [w u32][h u32][jpeg]
                Page->>Session: ping (every 5 s) / tap / scroll / choose / park / start / mobile
                Session->>Channel: the matching RemoteIntent
                Window->>Session: publish_state(RemoteState) on every redraw
                Session-->>Page: state {…}
            end
            alt any other message type
                Session->>Session: tracing::warn!, no reply
            end
            alt leave received, socket closed, or 15 s without ping
                Session->>Channel: RemoteIntent::Leave (once)
            else revoke_phone() from the desktop
                Session->>Record: clear()
                Session-->>Page: bye {reason: revoked}, close
                Session->>Channel: RemoteIntent::Leave
            end
        end
    end
```

**Architecture rules**

- Rule 1 — the intent and state types live in the core with no serde; the
  wire types in `protocol.rs` map to and from them
- Rule 7 applied to the wire — the JSON message shapes are the adapter's own
  types, so a domain rename never changes the protocol
- Rule 8 — every phone message becomes a `RemoteIntent` the window applies
  through its existing handlers; the server decides nothing about accounts
- Rule 9 — `Presence::observe(now_millis)` decides the silence limit from
  elapsed time passed in
- Rule 10 — JPEG encoding and socket I/O run on the connection thread;
  `apply_remote_intent` runs on the GTK main context via the channel
- Rule 14 — the handshake, the proof and the closed message set are covered
  by integration tests against a TCP client
