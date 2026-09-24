# Update notice

## Purpose

Where the user learns a newer version exists, fetches it, and chooses the
moment it installs. Also where a manual check answers.

## Where it sits

Renders the `**Entry**` bullet's automatic appearance, every `**Flow**` bullet
from "A newer version exists" to "Check for updates from the main menu", the
`**States**` bullets for a failed manual check, a failed download, dismissal
and the strip-and-notice stacking, and all three interaction diagrams in
`## User Experience`.

## The screen

A bar directly under the header bar, spanning the sidebar and the grid, in the
theme's warning tint. When the message strip is also showing, the strip comes
first and the bar sits directly beneath it. Left to right: one line of text
that ellipsises at its end, a `What's new` link, one button, and a flat `×`.
The bar, the link's place and the button's place never move between states;
only the text, the link's visibility and the button's label and sensitivity
change. The states and their text:

| State | Line | Link | Button |
| --- | --- | --- | --- |
| Checking (manual only) | `Checking for updates…` | hidden | hidden |
| Up to date (manual only) | `You have the latest version, 0.2.0.` | hidden | hidden |
| Available | `Version 0.3.0 is available.` | shown | `Update` |
| Downloading | `Downloading version 0.3.0… 42%` | shown | `Update`, insensitive |
| Verifying | `Checking the download…` | shown | `Update`, insensitive |
| Ready | `Version 0.3.0 is ready. It installs when you quit Idle Manager.` | shown | `Restart now` |
| Failed, version known | `The update could not be verified and was discarded.` or `The download failed: <reason>.` | shown | `Try again` |
| Failed, check itself | `Could not check for updates: <reason>.` | hidden | hidden |

```
┌──────────────────────────────────────────────────────────────────────┐
│ ▤ ⟳            Melvor · Main               [1][2][4][📱]          ☰  │
├──────────────────────────────────────────────────────────────────────┤
│ The arrangement could not be saved: disk full.                    × │  ← message strip (rule 9), only when it has something to say
├──────────────────────────────────────────────────────────────────────┤
│ Version 0.3.0 is available.            What's new     [ Update ]  × │  ← update notice
├────────────┬─────────────────────────────────────────────────────────┤
│ sidebar    │ grid                                                    │

│ Downloading version 0.3.0… 42%         What's new     [ Update ]  × │  (button insensitive)

│ Version 0.3.0 is ready. It installs when you quit Idle Manager.      │
│                                        What's new  [ Restart now ] × │

│ You have the latest version, 0.2.0.                               × │
```

Dismissing hides the bar for the rest of the run. A `Ready` update still
installs on quit after its bar is dismissed. The bar reopens only when a later
check finds a still-newer version.

## Design rules

- Rule 9 — leaves only when dismissed by hand, warning tint never error red;
  by contrast, this bar carries an action, which rule 9 forbids the strip, so
  it is its own widget beneath the strip
- Rule 8 — a failed manual check is one line inside the thing the user asked
  for, and the rest of the window keeps working; never a modal
- Rule 12 — nothing moves between states: the same label, link and button keep
  their places, only their text and sensitivity change
- Rule 28 — the line ellipsises at its end; no fixed width on the bar or its
  button
- New pattern — a dismissible bar under the header bar with one line, a link
  and one state-labelled action; the design doc owes a rule once this ships
