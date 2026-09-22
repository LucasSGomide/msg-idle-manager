# Goal: Brief Claude Design to redesign Idle Manager's main window and dialogs — clearer icons, nothing important hidden, keyboard-first, and ready for Brazilian Portuguese

**Status:** not executed
**Rating:** —
**Run:** parallel with 10 — this prompt produces a design brief only and edits no code. Apply its output after 10 has landed: both touch the header bar and the shortcuts window.

## Context

You are designing for a **native Linux/Windows desktop application, not a web
app**. It is written in Rust with GTK 4, and every game it shows is a real
embedded browser view. That single fact governs the whole brief: the components
available are the ones the toolkit ships, and a redesign that assumes web
freedom cannot be built. The limits are spelled out under **Constraints** — read
them before drawing anything. You have no access to the source repository, so
everything you need is in this prompt.

**What the application is for.** One person runs several browser-based idle
games at once on a desktop that stays on all day. Each game is an *account* with
its own isolated browser storage, so two accounts of the same game never share a
login. Accounts are grouped into *workspaces*. The window shows one workspace at
a time in one of four arrangements — one game filling the window, two side by
side, four in a grid, or one game at a phone's own size (412 × 915). The
accounts of the shown workspace are read as *pages*: with four accounts in the
two-slot arrangement, page 1 is the first two and page 2 the next two. A sidebar
lists every account as a tree under its workspace heading. An account is either
*live* (running, on screen or out of sight), *parked* (not running at all, so it
costs no memory), *starting*, or *queued* (waiting its turn in a start queue that
starts one account at a time). Memory is the scarce resource — the sidebar
carries a running memory readout and warns when a budget is crossed.

**The problems to solve.** The owner's own words: "we lack some shortcuts, the
icons are not very straight forward and some elements are too hidden making it
difficult to use." Concretely, in today's build:

- **Almost every action lives behind a `⋯` menu.** Park/Start, Keep running when
  hidden, Rename and Delete are only reachable by opening an account row's `⋯`
  menu. `Park all` / `Start all` are only in a workspace heading's `⋯` menu. A
  slot showing a stopped account is the one exception: it has a visible `Start`
  button.
- **Moving an account to another workspace is a mode.** You press a `Select`
  toggle at the top of the sidebar, the rows grow checkboxes, you tick some, then
  press `Move to…`. Nothing hints the mode exists or what it is for.
- **The status vocabulary is six colours and no words.** A row's state is one
  10 px dot: green glowing (the focused account), green flat (on screen), amber
  (running out of sight), grey (parked), blue (starting), purple (queued). The
  word only appears as the dot's tooltip. A second, separate `◆` glyph at 45%
  opacity means "keep running when hidden".
- **Icons carry no words.** The header bar holds a bare `⟳` (reload — but of
  *which* game?), a `▤` sidebar toggle, and a `☰` menu that hides phone
  enrolment and the keyboard-shortcuts window. The arrangement toggles are
  labelled `1` `2` `4` `Phone` — numbers, not pictures of the arrangement.
- **Which game the keyboard and the actions apply to is faint.** The focused slot
  is marked by a 2 px outline and a bold sidebar row, and nothing else.
- **Zoom is invisible.** Each account remembers its own zoom level, changed only
  by `Ctrl`+`scroll` or `Ctrl`+`+`/`-`/`0`, acknowledged by a percentage that
  fades after a second. Nothing shows an account's current zoom at rest.
- **Drag to rearrange is hover-only.** A small grip appears in a slot's
  top-right corner on hover; dragging it onto another slot swaps the two.
- **There is no search, no filter, and no count** of accounts in a sidebar that
  will hold dozens.

**What is wanted back.** Annotated wireframes and design-rule changes — see
**Output**. Not pixel mockups: they would have to be re-derived as GTK widgets
by hand, and would promise things the toolkit cannot draw. You may rework the
window's layout — move or merge controls, add a row of chrome, re-place the
memory readout, change icons, promote hidden actions — as long as **every
capability listed in this brief stays reachable** and nothing needs a widget GTK
4.10 does not have. Do not propose a new navigation model, a command palette or
extra top-level windows; the scope is the window and dialogs described here.

**Brazilian Portuguese is part of the job.** The application is English-only
today, with no translation machinery at all. It is about to gain pt-BR, so the
redesign has to survive it: Portuguese labels run roughly 20–35% longer than
their English equivalents ("Add game" → "Adicionar jogo", "Keep running when
hidden" → "Continuar rodando quando oculto", "Move to…" → "Mover para…"). Any
layout that only works at English widths is not acceptable, and every visible
string you introduce has to be listed so it can be translated.

### Current state — the main window

Two-slot arrangement, sidebar shown, a message strip present (it appears only
after something failed without the user acting):

```
┌ Idle Manager ───────────────────────────────────────────────────────── ─ □ ✕ ┐
│ [⟳]                    [‹] 1/2 [›]  [1][2][4][Phone]  [☰] [▤] [ Add game ]   │
├──────────────────────────────────────────────────────────────────────────────┤
│ ⚠  Two accounts could not be restored — check their addresses.         [✕]   │
├───────────────────────┬──────────────────────────────────────────────────────┤
│ Accounts    [ Select ]│ ┌─────────────────────┐ ┌─────────────────────┐      │
│ ▾ Farming             │ │╔═══════════════════╗│ │                     │      │
│    Alt one      ● ⋯   │ │║                   ║│ │                     │      │
│    Alt two      ● ⋯   │ │║ live game page    ║│ │  live game page     │      │
│ ▾ Idle RPG            │ │║                   ║│ │                     │      │
│    Main       ● ◆ ⋯   │ │╚═══════════════════╝│ │                     │      │
│    Smurf        ● ⋯   │ │ ▲ 2 px outline =    │ │                     │      │
│ ▸ Ungrouped           │ │   the focused slot  │ │                     │      │
│                       │ └─────────────────────┘ └─────────────────────┘      │
│ ─────────────────────  │                                                     │
│ App             412 MiB│                                                     │
│ 3 running     1 204 MiB│                                                     │
│ Total         1 616 MiB│                                                     │
└───────────────────────┴──────────────────────────────────────────────────────┘
  200 px, fixed            the grid: 1, 2 or 4 equal slots, or one phone-sized
                           slot centred on the window background
```

Header bar, left to right: `⟳` reload (tooltip "Reload the focused game (F5)");
then, right-aligned, the pager `‹ n/m ›` (hidden whenever the workspace has one
page), the four arrangement toggles in one linked box, `☰` menu, `▤` sidebar
toggle, and `Add game` — the only labelled button in the bar.

### Current state — the sidebar

```
┌───────────────────────────┐   Row anatomy, left to right:
│ Accounts        [ Select ]│     [tick]  name … dot  ◆  ⋯
│───────────────────────────│   The tick appears in Select mode only.
│ ▾ Farming             ⋯   │   The name takes all remaining width and
│      Alt one        ● ⋯   │   ellipsises; dot, ◆ and ⋯ ride the
│      Alt two        ● ⋯   │   trailing edge.
│ ▾ Idle RPG            ⋯   │
│      Main       ● ◆ ⋯     │   Dots:  ● glowing green  focused account
│      Smurf          ● ⋯   │          ● green         on screen
│ ▾ Empty one           ⋯   │          ● amber         running, out of sight
│      No accounts          │          ● grey          parked
│ ▸ Ungrouped           ⋯   │          ● blue          starting
│                           │          ● purple        queued
│ ────────────────────────  │   ◆ = keep running when hidden (45% opacity)
│ App                412 MiB│   A heading carries no dot and no mark of
│ 3 running        1 204 MiB│   which workspace is on screen — only an
│ Total            1 616 MiB│   expander arrow and its own ⋯ menu.
│ over budget (2 048 MiB)   │   Names dim to 55% when parked, queued or
└───────────────────────────┘   out of sight; bold for the focused one.
```

Select mode, and the two `⋯` menus:

```
 Select mode                    Account row ⋯ menu      Workspace heading ⋯ menu
┌──────────────────────────┐   ┌────────────────────┐   ┌──────────────────────┐
│ Accounts        [ Done ] │   │ Park               │   │ Park all             │
│ ▾ Farming                │   │ Keep running when  │   │ Start all            │
│   [✓]  Alt one       ●   │   │   hidden        [ ]│   │──────────────────────│
│   [ ]  Alt two       ●   │   │ Rename…            │   │ Rename…              │
│ ▾ Idle RPG               │   │ Delete account…    │   │ Remove workspace     │
│   [ ]  Main          ●   │   └────────────────────┘   └──────────────────────┘
│──────────────────────────│    "Park" reads "Start"     Both top items grey out
│ 1 ticked     [ Move to…▾]│    once parked, and greys   when they would touch
└──────────────────────────┘    while starting.          no account. Ungrouped
   Headings lose their ⋯ menus   Every ⋯ menu is the      gets the top pair only.
   while the mode is on.         only way to these.
```

### Current state — a slot

```
 Live (Linux)              Parked / queued / starting     Zoom acknowledgement
┌──────────────────┐      ┌──────────────────────────┐   ┌──────────────────┐
│             [⠿]  │      │                          │   │                  │
│  the game's page │      │        Alt two           │   │  the game's page │
│  fills the slot  │      │        Parked            │   │                  │
│                  │      │       ( Start )          │   │      ┌───────┐   │
│                  │      │                          │   │      │ 110%  │   │
└──────────────────┘      └──────────────────────────┘   └──────┴───────┴───┘
 [⠿] drag grip, top         "Parked" + a Start button;     Flashes low and
 right, on hover only.      "Starting" and "Queued"        centred on a
 On Windows it moves to     show no button. Plain,         Ctrl+scroll or
 a thin strip above the     centred, on the window         Ctrl+±, fades after
 slot instead (see          background.                    ~1 s, leaves nothing.
 constraint 6).
```

Empty states: with no accounts at all the grid area shows "No games yet — add
one to get started." above a pill-shaped `Add your first game` button; a
workspace with no accounts shows "No games in this workspace".

### Current state — the dialogs

```
 Add game, stage 1              Add game, stage 2           Rename account
┌────────────────────────┐   ┌────────────────────────┐  ┌────────────────────┐
│ Add a game             │   │ Add a game             │  │ Rename account     │
│ Choose a game          │   │ Melvor Idle            │  │ ┌────────────────┐ │
│ ┌────────────────────┐ │   │ Name for this account  │  │ │ Main           │ │
│ │ Melvor Idle        │ │   │ ┌────────────────────┐ │  │ └────────────────┘ │
│ │ Cookie Clicker     │ │   │ │ Main account       │ │  │ Another workspace  │
│ │ Something else…    │ │   │ └────────────────────┘ │  │ has this name.     │
│ └────────────────────┘ │   │ Address                │  │                    │
│                        │   │ ┌────────────────────┐ │  │  [Cancel] [Rename] │
│                        │   │ │ https://…          │ │  └────────────────────┘
│                        │   │ └────────────────────┘ │
│                        │   │ Workspace  [ Farming ▾]│   Delete account
│            [ Cancel ]  │   │      [ Cancel ] [ Add ]│  ┌────────────────────┐
└────────────────────────┘   └────────────────────────┘  │ Delete account?    │
                                                         │ Delete "Main" and  │
 Phone                          Keyboard shortcuts       │ its saved data?    │
┌────────────────────────┐   ┌────────────────────────┐  │ [Cancel] [Delete]  │
│ Phone                  │   │ Idle Manager           │  └────────────────────┘
│ Waiting for the phone… │   │ Reload…          F5    │   Then a spinner, then
│ ┌──────────┐           │   │ Zoom in        Ctrl +  │   a failure page with a
│ │ QR code  │           │   │ Zoom out       Ctrl -  │   reason, a folder path
│ └──────────┘           │   │ Reset zoom     Ctrl 0  │   and [Close] [Retry].
│ https://…              │   │ Next account  Shift ⇥  │
│ expires in 2:41        │   │ Next workspace Ctrl ⇥  │   A one-line warning
│  [ Enrol… ] [Un-enrol] │   │ Shortcuts      Ctrl ?  │   strip can appear under
└────────────────────────┘   └────────────────────────┘   the header bar (⚠ text
   Reached from ☰ only.          Reached from ☰ or         + ✕), spanning sidebar
                                 Ctrl+? only.              and grid.
```

### Current state — the keys

`F5` / `Ctrl`+`R` reload the focused game · `Ctrl`+`+` / `-` / `0` zoom the
focused account (also `Ctrl`+`scroll` over it) · `Shift`+`Tab` next account
(turning the page at its end) · `Ctrl`+`Tab` next workspace · `Ctrl`+`?` the
shortcuts window. Every one of these is taken by the window before the game's
page can see it. About to be added by separate work, already decided, so design
around them: `Ctrl`+`B` show/hide the sidebar · `Ctrl`+`1` / `2` / `4` pick the
arrangement · `Ctrl`+`P` park and `Ctrl`+`S` start the focused account ·
`Ctrl`+`Shift`+`P` / `Ctrl`+`Shift`+`S` park or start the whole workspace.
Anything else you propose reaching by keyboard must say which chord, and must
not collide with these.

## Constraints

1. **GTK 4.10 with gtk4-rs, and no libadwaita.** The widget set is GTK's own:
   `GtkHeaderBar`, `GtkBox`, `GtkGrid`, `GtkPaned`, `GtkStack`, `GtkRevealer`,
   `GtkOverlay`, `GtkScrolledWindow`, `GtkListView` / `GtkColumnView` with
   `GtkTreeExpander`, `GtkFlowBox`, `GtkNotebook`, `GtkExpander`, `GtkPopover`,
   `GtkMenuButton`, `GtkButton` / `GtkToggleButton`, `GtkDropDown`, `GtkEntry` /
   `GtkSearchEntry`, `GtkCheckButton`, `GtkSwitch`, `GtkSpinner`,
   `GtkProgressBar`, `GtkLevelBar`, `GtkSeparator`, `GtkLabel`, `GtkPicture`,
   `GtkShortcutsWindow`. There is **no** libadwaita, so none of these exist:
   toasts, banners, `AdwActionRow` / `AdwPreferencesPage` rows, view switchers,
   `AdwTabBar` tabs, status pages, split views, bottom sheets, the accent-colour
   API. Anything you want that is not in the first list has to be assembled from
   boxes, labels and buttons — say so when you do, and keep the assembly shallow.
2. **GTK CSS is a small subset of web CSS.** Available: colours, backgrounds,
   borders, `border-radius`, `box-shadow`, padding, margin, `min-width` /
   `min-height`, font properties, `opacity`, `transition`, and the theme's named
   colours (`@window_bg_color`, `@warning_color`, …). **Not** available: any
   flexbox or grid layout (layout is what the widget tree does, not what CSS
   says), `::before` / `::after` pseudo-elements, generated content, arbitrary
   selectors, `z-index`, web fonts, filters and blurs, custom-property theming.
   Do not describe a visual that needs a stacking trick or a pseudo-element.
3. **Icons come from the installed symbolic icon theme** (Adwaita names such as
   `view-refresh-symbolic`, `go-next-symbolic`, `sidebar-show-symbolic`,
   `view-more-symbolic`, `media-playback-start-symbolic`,
   `media-playback-pause-symbolic`). They are single-colour, one flat size, and
   recoloured by the theme. When you ask for an icon, **name the icon** you mean,
   and never rely on colour inside the glyph or on a two-tone or custom-drawn
   mark; a bespoke icon would have to be commissioned and shipped, so treat it as
   a cost and say so.
4. **Light and dark, and no accent colour.** The design must read in both
   themes using theme-named colours, plus at most the small fixed palette the
   status dots already use. There is no system accent colour to follow.
5. **Menus are `GMenu` models.** A `GtkPopoverMenu` item is a label, optionally
   with an accelerator string, a checkmark or a radio dot, in sections. It cannot
   hold arbitrary widgets, a second line of description, or an icon in the
   general case. A richer popover is possible, but it stops being a menu and has
   to be built as a `GtkPopover` full of widgets — call that out where you want
   it.
6. **The game area is a native browser view, and on Windows it wins the
   airspace.** On Linux the view is a GTK widget, so GTK can draw over it. On
   Windows it is a `WebView2` child window that the platform's compositor draws
   **above anything GTK overlays in the same rectangle** — an overlaid badge,
   button or label there is simply invisible. So: anything that must be seen over
   a running game either lives in a row of real chrome beside the game (the
   existing drag grip does exactly this on Windows) or in a `GtkPopover`, which
   gets its own native surface. Never put a permanent control on top of a game.
   Assume roughly a 1 : 1 ratio of screen given to games over chrome: the games
   are the point, and chrome that eats the grid is chrome that costs the owner
   money in monitor space.
7. **One window, no new top-level windows** beyond the dialogs that exist
   (`Add game`, `Rename`, `Delete account`, `Phone`, the shortcuts window).
   Dialogs are modal, sized by their content, and must keep working at a small
   window size. There is no tray icon and no notification surface.
8. **Keep every capability reachable.** The full list: add an account; rename;
   delete; park / start one account; park / start a whole workspace; toggle
   "keep running when hidden"; move accounts between workspaces; rename or remove
   a workspace; reorder accounts by dragging; focus an account; turn the page;
   pick one of the four arrangements; reload the focused game; zoom the focused
   account in / out / reset; show and hide the sidebar; enrol or un-enrol a
   phone; open the shortcuts window; read the memory figures and the
   over-budget warning. Nothing on that list may become unreachable, and each
   should be reachable in at most two steps from the main window.
9. **The status vocabulary may be re-presented but not reduced.** Six states —
   focused, on screen, running out of sight, parked, starting, queued — plus the
   independent "keep running when hidden" flag, must each stay tellable apart at
   a glance, and the state word must be available to a screen reader. Colour
   alone as the only channel is the thing to fix; adding a word, a shape or a
   glyph is welcome as long as a row still fits a 200 px sidebar and the name
   still gets most of the width.
10. **Every string must survive pt-BR.** Assume 35% growth over English, wrap or
    ellipsise deliberately, never centre a label whose width decides a layout,
    and give every new string you introduce in a list at the end (English +
    suggested pt-BR), including tooltips.
11. **Respect, or explicitly amend, the existing design rules.** The project
    keeps a numbered rule file; the rules that bear on this work are summarised
    under **Examples**. Where your design keeps a rule, do not restate it; where
    it contradicts one, name the rule number, say what it should now say, and say
    why the change is worth breaking a recorded decision for. New patterns need a
    new rule written in the same shape: one imperative, one line of why.
12. **Say what you are unsure the toolkit can do.** If a piece of the design
    depends on behaviour you cannot confirm from this brief — a widget behaving a
    certain way, an icon existing, a gesture being available — flag it as
    "needs verifying" rather than assuming it works. A design the team cannot
    build is worse than a plainer one it can.

## Tone

Direct and concrete, plain words over design jargon. Each recommendation is one
imperative sentence and one line of why — the why is what makes it a rule rather
than a preference. No praise for the current design and no apology for changing
it; say what changes, where it sits, and what it costs.

## Output

A single markdown document, structured as:

1. **What is wrong now** — the problems you found, ranked by how much they cost
   the owner day to day. Ground each one in something in this brief, and name
   the ones you think are not worth fixing.
2. **The redesign, screen by screen** — one section per screen, each with a
   monospaced wireframe in the format shown under **Examples**: purpose, where it
   sits, the screen in prose, the ASCII drawing, and the rules it follows.
   Cover at least: the main window as a whole; the header bar; the sidebar
   (rows, headings, Select mode, the memory readout); a slot in each of its
   states; the row and heading menus; the `Add game` dialog's two stages;
   `Rename`; `Delete account`; `Phone`; the shortcuts window; and the empty and
   warning states.
3. **Icons and affordances** — a table of every icon in the redesigned UI: where
   it sits, the Adwaita icon name, what it means, its tooltip (English and
   pt-BR), and the key it mirrors if it has one.
4. **Keyboard** — every chord in the redesigned UI, including the ones listed
   above as already decided, and any you add.
5. **Design rules** — the numbered rules your design keeps, the ones it amends
   (with the number, the new wording and the reason), and the new ones it needs.
6. **Strings** — every visible string in the redesign, English and suggested
   pt-BR, grouped by screen.
7. **Needs verifying** — the list from constraint 12: each assumption about the
   toolkit that a developer must confirm before building.

## Examples

**The wireframe format to follow** — an existing wireframe from this project,
reproduced so your output can be dropped straight in beside it:

~~~markdown
# Header-bar pager

## Purpose

Where the owner sees which page of the shown workspace is on screen and turns
to the next or previous one with the mouse.

## Where it sits

Renders the pager entry, the "Turn a page" and "Change layout" flows, and the
"One page" state.

## The screen

The existing header bar, unchanged except for one linked box directly left of
the `1` `2` `4` `Phone` layout toggles: a `‹` icon button, an `n/m` readout in
tabular figures, and a `›` icon button. The box is present only while the shown
workspace has more than one page…

```
┌────────────────────────────────────────────────────────────────────────┐
│ [⟳]              [‹] 2/3 [›] [1][2][4][Phone] [☰] [▤] [Add game]       │
└────────────────────────────────────────────────────────────────────────┘
   ▲ reload         ▲ pager: hidden when m == 1
```

## Design rules

- Rule 11 — the readout is a fixed-width numeric figure so a digit changing
  never shifts its neighbour
- Rule 13 — a page turn re-keys the sidebar rows; no heading gains a mark of
  the page
~~~

**The design rules now on record**, in the shape new ones must take — one
imperative, one line of why. Numbers are permanent; cite them when you amend
one:

1. Mark a sidebar row's state with one coloured dot and nothing else visible;
   carry the state word only as the dot's tooltip and accessible label. The
   marker must carry a growing number of facts, and dropping the always-visible
   word hands the trailing edge back the width a name was losing to it.
2. Give a row's Park/Start action one control whose label and effect invert with
   the row's state, and fold it into the row's `⋯` menu rather than standing a
   button on the row. The action is always "move this account to the other
   liveness", so one control can never be pressed in a direction that does not
   apply — and the trailing edge is scarce.
3. Dim a row's name to 55% alpha whenever the account is out of sight or parked,
   and keep bold for the focused row alone. Both mean "not something you are
   watching right now", so they take one style rather than two the eye must learn.
4. Stand in for an absent game with a plain centred panel on the window's own
   background — name, state word, and one action button when an action applies.
5. Put both the set-and-forget settings a row carries and its destructive
   actions behind the row's `⋯` menu, each one deliberate.
6. When a row's trailing edge is asked to carry more than one fact, let the
   quieter one be a small glyph rather than a second dot.
7. Set an escape-hatch option apart from the real options in a chooser with a
   separator, so "Something else…" never reads as a game.
8. Report a partial failure inside a form as one dim line beneath the field it
   belongs to.
9. Say what went wrong before the user did anything in one strip across the top
   of the content, tinted with the theme's warning colour and dismissible.
10. Acknowledge a direct gesture with a transient figure drawn over the place it
    affected, which fades on a timer and leaves nothing behind; never draw one
    for a change the user did not ask for.
11. Show a measured figure right-aligned to a shared edge, in the smaller type
    at a fixed-width numeric style, and draw an unmeasured one as an en dash —
    never a zero.
12. A readout that changes appearance because a measurement crossed a line must
    clear itself when a later measurement crosses back, and must offer no action.
13. Give a sidebar tree heading no dot, no bold and no mark of which workspace is
    shown — the accounts nested beneath it carry every status signal.
14. A control that must reach the user over a native child window the platform's
    own engine draws gets its own row of chrome next to the content instead of an
    overlay on top of it; a transient figure that must win that airspace gets its
    own native surface. (This is constraint 6 above, as a rule.)
15. A slot that stands in for another screen keeps that screen's size: centred,
    outlined, and clipped by the window rather than scaled to fit it.
16. Pair a device with one screen that shows the same secret two ways for a fixed
    time and says where things stand on its first line.
17. A page the application serves to another device follows these rules where the
    medium allows and states where it departs.
18. Show a workspace's pages as two icon-button arrows around an `n/m` readout,
    joined in one linked box beside the controls it pages through, and hide the
    whole box the moment there is only one page rather than grey it.
19. List every key the window answers in one shortcuts window opened by
    `Ctrl`+`?` and a `Keyboard Shortcuts` menu item — and name a control's own key
    in its tooltip wherever the control mirrors one.
