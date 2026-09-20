//! The wire: the JSON shapes both sides exchange, as `openapi.json` describes
//! them, and their mapping to and from the core's vocabulary.
//!
//! These are the adapter's own types (architecture rule 7 applied to the
//! wire): a rename in `idle_manager_core::remote` changes the `From` and
//! `TryFrom` below and never a byte the phone sees.

use serde::{Deserialize, Serialize};

use idle_manager_core::{
    Liveness, RemoteAccount, RemoteIntent, RemoteState, RemoteWorkspace, SessionId, Viewport,
};

/// Every `type` the phone may send, in `openapi.json`'s order. Kept beside the
/// enum so a message whose type is missing here is reported as unknown rather
/// than malformed.
const PHONE_TYPES: [&str; 10] = [
    "auth", "attach", "leave", "ping", "choose", "park", "start", "mobile", "tap", "scroll",
];

/// Why a text message from the phone was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum WireError {
    /// The text is not a JSON document.
    #[error("the message is not JSON: {reason}")]
    NotJson {
        /// What the parser objected to.
        reason: String,
    },
    /// The document has no string-valued `type`.
    #[error("the message carries no `type`")]
    NoType,
    /// The `type` is none the desktop understands (`FR.6.3`).
    #[error("the message type {kind:?} is not one the desktop understands")]
    UnknownType {
        /// The `type` as the phone sent it.
        kind: String,
    },
    /// The `type` is known but a field is missing or of the wrong shape.
    #[error("the {kind} message is malformed: {reason}")]
    Malformed {
        /// The `type` the message named.
        kind: String,
        /// What the parser objected to.
        reason: String,
    },
    /// A message that belongs to the session, not to the application.
    #[error("a {kind} message is not an intent")]
    NotAnIntent {
        /// The `type` of the message.
        kind: &'static str,
    },
}

/// A phone screen's size, as `Viewport` on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WireViewport {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl From<Viewport> for WireViewport {
    fn from(viewport: Viewport) -> Self {
        Self {
            width: viewport.width,
            height: viewport.height,
        }
    }
}

impl From<WireViewport> for Viewport {
    fn from(viewport: WireViewport) -> Self {
        Self {
            width: viewport.width,
            height: viewport.height,
        }
    }
}

/// `Liveness` on the wire: the sidebar's own words (design rule 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WireLiveness {
    Live,
    Parked,
    Starting,
    Queued,
}

impl From<Liveness> for WireLiveness {
    fn from(liveness: Liveness) -> Self {
        match liveness {
            Liveness::Live => Self::Live,
            Liveness::Parked => Self::Parked,
            Liveness::Starting => Self::Starting,
            Liveness::Queued => Self::Queued,
        }
    }
}

/// `RemoteAccount` on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WireAccount {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) liveness: WireLiveness,
}

impl From<&RemoteAccount> for WireAccount {
    fn from(account: &RemoteAccount) -> Self {
        Self {
            id: account.id.as_str().to_owned(),
            name: account.name.clone(),
            liveness: account.liveness.into(),
        }
    }
}

/// `RemoteWorkspace` on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WireWorkspace {
    pub(crate) name: String,
    pub(crate) accounts: Vec<WireAccount>,
}

impl From<&RemoteWorkspace> for WireWorkspace {
    fn from(workspace: &RemoteWorkspace) -> Self {
        Self {
            name: workspace.name.clone(),
            accounts: workspace.accounts.iter().map(WireAccount::from).collect(),
        }
    }
}

/// `RemoteState` on the wire, camel-cased as `openapi.json` spells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WireState {
    pub(crate) mobile_mode: bool,
    pub(crate) viewport: WireViewport,
    pub(crate) current: Option<String>,
    pub(crate) workspaces: Vec<WireWorkspace>,
}

impl From<&RemoteState> for WireState {
    fn from(state: &RemoteState) -> Self {
        Self {
            mobile_mode: state.mobile_mode,
            viewport: state.viewport.into(),
            current: state.current.as_ref().map(|id| id.as_str().to_owned()),
            workspaces: state.workspaces.iter().map(WireWorkspace::from).collect(),
        }
    }
}

/// Why the server is closing the socket, in `ServerBye`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ByeReason {
    /// The desktop forgot this phone (`FR.6.2`).
    Revoked,
    /// A newer socket from the same phone took over.
    Replaced,
}

/// Every message the server sends: the `Server*` schemas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum ServerMessage {
    Hello { challenge: String },
    Welcome { proof: String, state: WireState },
    State { state: WireState },
    Bye { reason: ByeReason },
}

impl ServerMessage {
    /// The message as the JSON text that goes on the wire.
    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string(self)
            .expect("wire messages are plain structs of strings, numbers and booleans")
    }
}

/// Every message the phone may send: the `Phone*` schemas, plus
/// [`PhoneMessage::Unknown`] for a `type` outside that closed set so the
/// session can log which one arrived.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum PhoneMessage {
    Auth {
        proof: String,
        challenge: String,
    },
    Attach {
        viewport: WireViewport,
    },
    Leave,
    Ping,
    Choose {
        account: String,
    },
    Park {
        account: String,
    },
    Start {
        account: String,
    },
    Mobile {
        on: bool,
    },
    Tap {
        x: u32,
        y: u32,
    },
    Scroll {
        x: u32,
        y: u32,
        dx: i32,
        dy: i32,
    },
    /// A `type` the desktop does not understand; never produced by serde,
    /// only by [`PhoneMessage::parse`].
    #[serde(skip)]
    Unknown {
        kind: String,
    },
}

impl PhoneMessage {
    /// Parses one text message from the phone.
    ///
    /// # Errors
    ///
    /// [`WireError::NotJson`] or [`WireError::NoType`] when the text has no
    /// usable `type`; [`WireError::Malformed`] when a known type's fields are
    /// wrong. An unknown type is `Ok(PhoneMessage::Unknown)`, not an error, so
    /// the caller can tell "the phone spoke out of turn" from "garbage".
    pub(crate) fn parse(text: &str) -> Result<Self, WireError> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|error| WireError::NotJson {
                reason: error.to_string(),
            })?;
        let kind = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or(WireError::NoType)?
            .to_owned();
        if !PHONE_TYPES.contains(&kind.as_str()) {
            return Ok(Self::Unknown { kind });
        }
        serde_json::from_value(value).map_err(|error| WireError::Malformed {
            kind,
            reason: error.to_string(),
        })
    }
}

impl TryFrom<PhoneMessage> for RemoteIntent {
    type Error = WireError;

    fn try_from(message: PhoneMessage) -> Result<Self, Self::Error> {
        match message {
            PhoneMessage::Attach { viewport } => Ok(Self::Attach {
                viewport: viewport.into(),
            }),
            PhoneMessage::Leave => Ok(Self::Leave),
            PhoneMessage::Choose { account } => Ok(Self::ChooseAccount(SessionId::new(account))),
            PhoneMessage::Park { account } => Ok(Self::Park(SessionId::new(account))),
            PhoneMessage::Start { account } => Ok(Self::Start(SessionId::new(account))),
            PhoneMessage::Mobile { on } => Ok(Self::SetMobileMode(on)),
            PhoneMessage::Tap { x, y } => Ok(Self::Tap { x, y }),
            PhoneMessage::Scroll { x, y, dx, dy } => Ok(Self::Scroll { x, y, dx, dy }),
            PhoneMessage::Auth { .. } => Err(WireError::NotAnIntent { kind: "auth" }),
            PhoneMessage::Ping => Err(WireError::NotAnIntent { kind: "ping" }),
            PhoneMessage::Unknown { kind } => Err(WireError::UnknownType { kind }),
        }
    }
}

/// Lowercase hex of `bytes`, the spelling every code, id and proof uses.
pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0x0F)],
            ]
        })
        .map(char::from)
        .collect()
}

/// The bytes `text` spells in hex, either case; `None` on an odd length or a
/// non-hex digit.
pub(crate) fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(text: &str) -> Result<RemoteIntent, WireError> {
        RemoteIntent::try_from(PhoneMessage::parse(text).expect("the text parses"))
    }

    #[test]
    fn every_intent_message_maps_to_its_remote_intent() {
        let cases = [
            (
                r#"{"type":"attach","viewport":{"width":412,"height":915}}"#,
                RemoteIntent::Attach {
                    viewport: Viewport {
                        width: 412,
                        height: 915,
                    },
                },
            ),
            (r#"{"type":"leave"}"#, RemoteIntent::Leave),
            (
                r#"{"type":"choose","account":"a1"}"#,
                RemoteIntent::ChooseAccount(SessionId::new("a1")),
            ),
            (
                r#"{"type":"park","account":"a2"}"#,
                RemoteIntent::Park(SessionId::new("a2")),
            ),
            (
                r#"{"type":"start","account":"a3"}"#,
                RemoteIntent::Start(SessionId::new("a3")),
            ),
            (
                r#"{"type":"mobile","on":true}"#,
                RemoteIntent::SetMobileMode(true),
            ),
            (
                r#"{"type":"tap","x":1,"y":2}"#,
                RemoteIntent::Tap { x: 1, y: 2 },
            ),
            (
                r#"{"type":"scroll","x":3,"y":4,"dx":-5,"dy":6}"#,
                RemoteIntent::Scroll {
                    x: 3,
                    y: 4,
                    dx: -5,
                    dy: 6,
                },
            ),
        ];

        let mapped: Vec<_> = cases.iter().map(|(text, _)| intent(text)).collect();

        let expected: Vec<_> = cases.into_iter().map(|(_, intent)| Ok(intent)).collect();
        assert_eq!(mapped, expected);
    }

    #[test]
    fn auth_and_ping_belong_to_the_session_and_are_not_intents() {
        let refused = [
            intent(r#"{"type":"auth","proof":"00","challenge":"11"}"#),
            intent(r#"{"type":"ping"}"#),
        ];

        assert_eq!(
            refused,
            [
                Err(WireError::NotAnIntent { kind: "auth" }),
                Err(WireError::NotAnIntent { kind: "ping" })
            ]
        );
    }

    #[test]
    fn an_unknown_type_is_refused_with_its_name() {
        let refused = intent(r#"{"type":"rename","account":"a1","name":"x"}"#);

        assert_eq!(
            refused,
            Err(WireError::UnknownType {
                kind: "rename".to_owned()
            })
        );
    }

    #[test]
    fn a_known_type_with_a_missing_field_is_malformed_not_unknown() {
        let refused = PhoneMessage::parse(r#"{"type":"tap","x":1}"#);

        assert!(matches!(refused, Err(WireError::Malformed { kind, .. }) if kind == "tap"));
    }

    #[test]
    fn text_without_a_type_or_not_json_is_refused() {
        let refused = [
            PhoneMessage::parse(r#"{"x":1}"#),
            PhoneMessage::parse("not json"),
        ];

        assert!(matches!(refused[0], Err(WireError::NoType)));
        assert!(matches!(refused[1], Err(WireError::NotJson { .. })));
    }

    #[test]
    fn the_phone_type_list_names_every_variant_serde_accepts() {
        let parsed: Vec<_> = PHONE_TYPES
            .iter()
            .map(|kind| PhoneMessage::parse(&format!(r#"{{"type":"{kind}"}}"#)))
            .collect();

        let any_unknown = parsed
            .iter()
            .any(|outcome| matches!(outcome, Ok(PhoneMessage::Unknown { .. })));
        assert!(!any_unknown, "{parsed:?}");
    }

    #[test]
    fn the_state_is_camel_cased_with_the_liveness_words() {
        let state = RemoteState {
            mobile_mode: true,
            viewport: Viewport {
                width: 400,
                height: 800,
            },
            current: Some(SessionId::new("s1")),
            workspaces: vec![RemoteWorkspace {
                name: "Main".to_owned(),
                accounts: vec![RemoteAccount {
                    id: SessionId::new("s1"),
                    name: "Alpha".to_owned(),
                    liveness: Liveness::Queued,
                }],
            }],
        };

        let json = ServerMessage::State {
            state: WireState::from(&state),
        }
        .to_json();

        assert_eq!(
            json,
            r#"{"type":"state","state":{"mobileMode":true,"viewport":{"width":400,"height":800},"current":"s1","workspaces":[{"name":"Main","accounts":[{"id":"s1","name":"Alpha","liveness":"queued"}]}]}}"#
        );
    }

    #[test]
    fn hex_round_trips_and_rejects_odd_or_non_hex_text() {
        let encoded = encode_hex(&[0, 15, 255]);

        assert_eq!(
            (
                encoded.as_str(),
                decode_hex(&encoded),
                decode_hex("abc"),
                decode_hex("zz")
            ),
            ("000fff", Some(vec![0, 15, 255]), None, None)
        );
    }
}
