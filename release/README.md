# The release signing key

`minisign.pub` beside this file is one half of the key pair that signs every
release asset (roadmap item 16, task 04). It is safe to commit: a public key
only lets you *verify* a signature, never produce one.

The pair was made once, with:

```
minisign -G -W -p release/minisign.pub -s <private>
```

`-W` skips a password on the secret key, because `make release-sign` runs it
unattended in CI. `<private>` is never written into the repository or a CI
log — it becomes the `MINISIGN_SECRET_KEY` Actions secret, and
`make release-sign` (`scripts/release-sign.sh` via the Makefile) writes it to
a temporary file named by `MINISIGN_SECRET_KEY_FILE` for the one command that
needs it, then the workflow step that created it deletes it.

`crates/idle-manager-update/src/signature.rs` compiles `minisign.pub` in with
`include_str!` and checks every downloaded package against it before the
update channel applies anything (task 06).

**Rotating the key** means generating a new pair the same way, committing the
new `minisign.pub` in a version the update channel will actually reach, and
replacing the `MINISIGN_SECRET_KEY` secret — a version signed with the old key
stops verifying once the new public key ships, so rotate only alongside a
release built from the new commit.
