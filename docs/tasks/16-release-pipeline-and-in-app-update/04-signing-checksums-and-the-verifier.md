# 04 — Signing, checksums and the verifier

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** back-end · **Depends on:** 03

## Context

When the app downloads a new version of itself and installs it, the person
running it is trusting that file completely. A checksum proves the download
arrived intact, but anyone who could tamper with the release page could
change the checksum too. A signature is different: the owner signs each
package with a private key that lives only in the project's GitHub secrets,
and the app carries the matching public key compiled into it. A package that
was not signed with that key is refused, whatever the release page says.

This slice adds both layers. Two commands sign every package in the release
folder and write a checksum file over all of them; the publish workflow later
runs them. The signing tool is minisign, a small standard whose signatures are
one short text file beside each package. The public key is committed to the
repository, and a tiny verification module in the update crate reads it,
checks a downloaded package against its signature file, and deletes the
package when the check fails. The module is tested with a throwaway key pair:
a good signature passes, a signature from a different key fails, and a
package changed by one byte fails.

Nothing here reaches the network or the screen yet; the next slice's update
channel calls the verifier between downloading and applying. Provenance
attestations, the third layer, are a workflow step and belong to the publish
slice.

It is its own slice because the key handling and the verifier are small,
security-sensitive and testable on their own, and because the update channel
and the publish workflow both need them finished first.

## Technical details

- **Architecture** — `release/minisign.pub`: the committed public key, with
  a `release/README.md` saying how the pair was made (`minisign -G -W -p
  release/minisign.pub -s <private>`), that the private key is the
  `MINISIGN_SECRET_KEY` Actions secret, and that rotating it means a new
  version carrying the new public key.
- **Architecture** — Makefile targets `release-sign` (`for f in
  dist/releases/*/*; do minisign -S -s "$MINISIGN_SECRET_KEY_FILE" -m "$f";
  done`, skipping existing `.minisig` files, failing when the variable is
  unset) and `release-checksums` (`cd dist/releases && sha256sum */* >
  SHA256SUMS`), both `.PHONY` with help lines; `scripts/system-check.sh` names
  `minisign` (`apt install minisign`).
- **Architecture** — `crates/idle-manager-update/src/signature.rs`:
  `pub(crate) const PUBLIC_KEY: &str = include_str!("../../../release/minisign.pub")`,
  `pub(crate) fn verify_package(package: &Path, signature: &Path) ->
  Result<(), SignatureError>` using `minisign-verify` 0.2 (added to the crate
  and the workspace root), reading the whole file, and deleting `package` on
  `Err`; `SignatureError` is a `thiserror` enum with `Unreadable { path }`,
  `MalformedSignature`, `Rejected` (rule 11).
- **Architecture** — `tests/fixtures/` in the update crate holds a test key
  pair, a small package file, its valid `.minisig`, a `.minisig` from another
  key and a tampered copy of the package; `tests/signature-rejects-what-the-key-did-not-sign.rs`
  covers the three cases (naming rule 3).
- **Code standards** — the public key constant and `verify_package` carry
  `///` contracts stating what a pass proves and what it does not (rule 17);
  no `unwrap` outside the tests (rule 13); the deletion on failure is logged
  with `tracing::warn!` in fields (rule 15).

## Acceptance criteria

- [ ] `(integration)` `verify_package` returns `Ok(())` for the fixture package
      and its valid signature, and the file is still there afterwards
- [ ] `(integration)` `verify_package` returns `SignatureError::Rejected` for a
      signature made with a different key, and the package file has been
      deleted
- [ ] `(integration)` `verify_package` returns `SignatureError::Rejected` for
      the tampered package with the original signature, and the file has been
      deleted
- [ ] `(integration)` `verify_package` returns `SignatureError::Unreadable`
      naming the path when the signature file is missing
- [ ] `(integration)` after `make windows-package` and `make linux-package`,
      `MINISIGN_SECRET_KEY_FILE=<test key> make release-sign` writes one
      `.minisig` beside every file under `dist/releases/*/`, and `minisign -V
      -p release/minisign.pub -m <file>` passes for each when signed with the
      committed key's pair
- [ ] `(integration)` `make release-checksums` writes `dist/releases/SHA256SUMS`
      with one line per asset and `sha256sum -c SHA256SUMS` passes
- [ ] `(integration)` `make verify` passes

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
  — Back-end "Signing and checksums"; Technical References on minisign
- [`docs/requirements.md`](../../requirements.md) — Distribution `FR.5.1`,
  `FR.5.2`
- [`docs/architecture.md`](../../architecture.md) — rules 11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 12, 13, 15, 17
- [`docs/naming.md`](../../naming.md) — rules 1, 3

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
