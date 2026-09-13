# Goal: Investigate and fix the CPU spike to 100% when 4 Hunter's Era sessions are logged in and playing

**Status:** not executed
**Rating:** —

## Context
With 4 characters logged into Hunter's Era (current sessions saved) and actively
playing, CPU usage spikes to 100%. At the login screen, before characters are
playing, CPU stays normal — the spike is specific to Hunter's Era running as a
game, not general app overhead. It's unclear whether this is a longstanding
cost of running 4 sessions or a regression, but it feels like it has gotten
more laggy recently — recent work landed on zoom handling (roadmap 09) and
memory accounting (roadmap 05), either of which could be implicated. Investigate
whether the app is leaking resources (timers, event handlers, WebKit views not
being cleaned up, redundant polling/rendering per session) or otherwise burning
CPU without need, then fix it.

## Constraints
1. Fully autonomous — no manual work handed back to the user; investigate and
   fix without asking them to read code, add logging, or interpret profiles
   themselves.
2. No regressions on any UI feature.
3. No session loss — every one of the 4 saved/logged-in sessions must survive
   the investigation and fix intact.
4. The user may be asked to start or stop the application to aid investigation
   (e.g. reproduce the spike, confirm a fix) — that's the extent of manual
   involvement expected from them.

## Output
Fix the code directly. No written findings report — summarize what was found
and changed in chat only, minimal explanation.
