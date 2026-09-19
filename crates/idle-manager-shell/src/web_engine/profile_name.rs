//! Whether a session identifier is a valid `WebView2` profile name.
//!
//! Pure and dependency-free on purpose: `make windows-check` only compiles
//! and lints the Windows target, it never links or runs it, so this is the
//! one piece of the Windows engine seam this machine can actually execute a
//! test against (architecture rule 14). `cfg(any(windows, test))` compiles it
//! for the real Windows backend and for `cargo test` on Linux alike, and
//! nowhere else — a Linux release build never carries it. Gated at the `mod`
//! declaration in `web_engine.rs`, not here.

/// `WebView2`'s own limit on `with_profile_name` (Technical References): at
/// most 64 characters, each one an ASCII letter, digit, `.`, `_`, ` ` or `-`.
/// Every identifier `idle-manager-core`'s session book mints (`session-NNNN`)
/// is well inside it; this only guards the invariant in one place, so a
/// future change to how identifiers are minted fails loudly here rather than
/// inside a `WebView2` COM call with a far less readable error.
pub(crate) fn is_valid_profile_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b' ' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_identifier_the_session_book_mints_is_a_valid_profile_name() {
        for id in ["session-0001", "session-0042", "session-9999"] {
            assert!(is_valid_profile_name(id), "{id} should be valid");
        }
    }

    #[test]
    fn a_name_at_exactly_the_length_limit_is_valid() {
        let name = "a".repeat(64);

        assert!(is_valid_profile_name(&name));
    }

    #[test]
    fn a_name_one_character_past_the_limit_is_rejected() {
        let name = "a".repeat(65);

        assert!(!is_valid_profile_name(&name));
    }

    #[test]
    fn a_name_containing_a_path_separator_is_rejected() {
        assert!(!is_valid_profile_name("session/0001"));
    }

    #[test]
    fn an_empty_name_is_rejected() {
        assert!(!is_valid_profile_name(""));
    }
}
