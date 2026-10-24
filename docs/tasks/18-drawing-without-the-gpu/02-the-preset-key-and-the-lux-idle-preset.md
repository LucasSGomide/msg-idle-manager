# 02 — The preset key and the shipped Lux Idle preset

**Roadmap:** [18](../../roadmap/18-drawing-without-the-gpu/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

A preset is a small hand-editable file describing one game: its name, its
address, its zoom and a few switches. When the owner adds an account from a
preset, the account copies those settings. The application ships three such
files and writes them into the owner's preset folder the first time that folder
is missing.

This slice lets a preset ask for the new per-account switch that makes the web
engine draw a game with the processor instead of the graphics chip. A preset
file may now say `software_rendering = true`; an account added from it starts
with the switch on. A file that leaves the key out means off, so the three
existing game files behave exactly as before. A file whose value is not true or
false still loads, with the switch off and one warning in the log naming the
file, which is the same tolerance the existing WebGL key has: one mistyped
switch should not hide a whole game from the chooser.

It also adds a fourth shipped preset, for Lux Idle, with the switch on. Lux Idle
is the game whose list of hunting grounds made the engine's graphics drawing
consume memory until the laptop froze; the file's comment says so and names the
measurement, so whoever reads it later knows the switch is there on purpose.

It is a separate slice because it touches only the preset reading code and the
preset files, while the interface slice touches only the interface, so the two
can be built at the same time once the stored flag exists.

## Technical details

- **Architecture** — the store's `PresetFile` owns the on-disk key and maps it
  to the domain `Preset` (architecture rule 7); the core names no file type
  (rule 1).
- **Code standards** — `Preset` in `idle-manager-core/src/preset.rs` gains
  `pub is_software_rendered: bool`; `SessionBook::add_from_preset` copies it
  onto the new session; preset fixtures gain `false`.
- **Code standards** — `PresetFile` in `idle-manager-store/src/preset.rs`
  gains `#[serde(default)] software_rendering: Option<toml::Value>`, read by a
  `software_rendered(value, entry) -> bool` mirroring `webgl_enabled` against a
  `SOFTWARE_RENDERING_DEFAULT: bool = false` constant (rule 5), with one
  `warn` naming the file for a non-boolean.
- **Code standards** — `presets/lux-idle.toml`: `name = "Lux Idle"`,
  `url = "https://luxidle.com.br/jogar"`, `zoom = 1.0`, `keep_awake = false`,
  `software_rendering = true`, with a comment naming roadmap item 18 and the
  2026-10-24 measurement (rule 18); appended to `SHIPPED_PRESETS` (the array
  length becomes 4).
- **Naming** — rule 1: the preset file name is kebab-case, `lux-idle.toml`.

## Acceptance criteria

- [x] `(unit)` a preset file with no `software_rendering` key reads as off
- [x] `(unit)` `software_rendering = true` reads back as on
- [x] `(unit)` a non-boolean value falls back to off
- [x] `(unit)` an account added from a preset with the flag on is software rendered, and one from a preset without it is not
- [x] `(integration)` a presets folder with one flagged and one unflagged file reads back with only the flagged one on, and a non-boolean value still loads the other fields
- [x] `(integration)` seeding a missing presets folder writes `lux-idle.toml` beside the other three, and it reads back as "Lux Idle" with the flag on

## References

- [Roadmap item](../../roadmap/18-drawing-without-the-gpu/README.md) — the full picture
- [Architecture](../../architecture.md), [Code standards](../../code-standards.md), [Naming](../../naming.md)

## Implement with

Direct implementation, TDD from the criteria.
