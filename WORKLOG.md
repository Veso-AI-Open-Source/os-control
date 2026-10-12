# OS Control worklog

This file records the work as it happens: what was built, what broke, how it was checked, and what comes next. A fresh session, or a new contributor, should be able to pick up from here without reading the whole history.

> **Newest entry first.** To add a session, copy the template below, fill it in, and put it above the most recent entry. Do not edit older entries.

<!--
ENTRY TEMPLATE. Copy from here, paste ABOVE the newest entry, fill in.
Delete any section that does not apply. Keep the order. Use absolute dates
(YYYY-MM-DD). State facts: real numbers, honest failures.

# Worklog - YYYY-MM-DD - <short title: the one thing this session was about>

> One paragraph: what this session set out to do and why. If it split into
> arcs, name them here.

## TL;DR
- The headline outcome in one or two sentences: what shipped, what is blocked.
- Three to six bullets: the concrete wins, each with a number where one exists.
- If blocked, say exactly what by.

## Threads / detail
One "### <thread name>" per line of work. For each: the problem, the decision
made, the alternative rejected and why, and the files touched.

## Snag log
The honest failures: what broke, dead ends, tooling that fought back, things
deferred. Do not tidy this section. It is where the value builds up.

## Verification
Test counts as real numbers ("N green, M failing"), build and install status,
and every live check that was run. Say what was NOT checked.

## Next steps
In priority order. Lead with any blocker.

## Pointers
Where the next session picks up: files, docs, logs.
-->

# Worklog - 2026-10-12 - Built in a morning, used on a real job, then open sourced

> The session set out to give Claude Code eyes and hands on a Mac, with brakes the agent cannot reach. It ran in five arcs. Arc A: the tools. Arc B: the hard stops. Arc C: the bar. Arc D: a first real job done with the tools. Arc E: the public release.

## TL;DR

- Shipped and public: https://github.com/Veso-AI-Open-Source/os-control, Apache-2.0. Three parts: `osctl` (MCP server and command line, 8 tools), `osctl-ask` (the gate panel), and `os-bar` (the bar).
- The tools read the accessibility tree. Reading all 241 controls of a Chrome window takes 0.35 s.
- A real Claude Code run worked out 123 × 45 in Calculator in the background: 12 turns, 11 s, no permission prompts.
- The gate panel refuses a posted click and an accessibility press. A real click allows it. All three are checked.
- First real job, in a web portal: 38 tool calls, 3.2 s inside the tools, 3 min 42 s from first call to last. It staged the change and stopped before Save.
- 44 unit tests are green. The live checks pass. A fresh clone builds in 1 min 13 s.
- Not done: the lessons from the first job (see Next steps), CI on GitHub, and a signed app bundle.

## Threads / detail

### Arc A: the tools

The shell was never the gap. Claude Code already runs commands. The gap was the screen, the pointer, and the keyboard.

- **Tree first, pixels second.** `see` walks the accessibility tree and returns each control with its role, label, and frame. A screenshot comes with it. Rejected: screenshots with guessed coordinates as the main path. A model that never states a position cannot state a wrong one, and the tree is faster.
- **Raw declarations, no binding crates.** `ffi.rs` declares every macOS C call by hand. The crate then has no dependency whose API can drift. The only outside crates in the core are `serde` and `serde_json`.
- **Presses go to the control.** `click` by id or label uses the press action of the control. The app stays in the background and the pointer does not move. The real pointer is the fallback.
- **One coordinate space.** Every x,y the model sees is a pixel in the last screenshot. One type, `View` in `geom.rs`, does every conversion.
- **Guards on keys and text.** Keys go to the app in front, so `key` and `type` take an `app` guard and refuse if another app is there. Typing checks the front app between chunks and stops if the focus moves.
- **A press on a point checks what is under it.** A position click with a guard is refused when another window covers the point.

Files: `crates/os-core/src/{ffi,cf,ax,input,screen,apps,geom,exec,tools}.rs`, `crates/osctl/src/{main,mcp}.rs`.

### Arc B: the hard stops

- **Decided in code.** `gate.rs` holds the rules for four gates: money, sending as you, passwords and keys, and destructive changes. It judges from the app, the web address, the label about to be clicked, and the key about to be pressed. The model has no vote.
- **Two kinds of rule.** A rule about what an act is, such as a Transfer button, asks every time. A rule about where the act happens, such as a bank's site, offers a yes for ten minutes.
- **Fail closed.** A `policy.json` that does not parse stops every tool.
- **The panel.** The first version used the system alert. It was dropped: it takes the keyboard, and its second button answers to Escape. The replacement is `helpers/ask.swift`, a panel that can never become the key window. Allow checks the event behind the click and accepts only one that came from hardware.
- **The stop switch.** One file in the state folder. Every tool checks it first. `osctl stop`, the bar, and the panel can all set it.

### Arc C: the bar

- A small native window built with GPUI, pinned to version 0.2.2 with runtime shaders, so the build needs no Metal toolchain.
- Each task is one headless Claude Code run that streams JSON. The bar turns each event into one short line. The next task resumes the same session.
- `OSBAR_AUTOSEND` runs scripted tasks at start, split by `||`. `OSBAR_QUIET` opens the bar without taking the focus. Both exist for checks and for the demo.

Files: `crates/os-bar/src/{main,run,theme}.rs`.

### Arc D: the first real job

The task was to find out how to play a short message to whoever answers a forwarded call. The tools did it in a web portal, through the command line, in a browser window that was mostly behind other windows.

- It read the routing page as text, opened a dropdown, found the setting, and staged it.
- The library held a recording with the right name. It downloaded the file and transcribed it to check what it said. A name is not proof of content.
- It did not press Save. The request was to find out, not to change.

What the job taught is the Next steps list below.

### Arc E: the public release

- Scrubbed every private name from code, comments, and tests.
- Apache-2.0, a notice file, topics, and a README written for strangers.
- The demo is recorded by `demo/record.py`. It captures three windows by their ids: the bar, a made-up invoice in Safari, and the gate panel. The rest of the screen never enters a frame.
- Fixes made on the way: icon glyphs no longer leak into labels, each control is listed once, a role alone can name a target, and a browser whose page tree comes back empty is asked again.

## Snag log

- **Typed text did not reach Calculator.** Key events that carried only a character were ignored. Some apps read the key, not the character. Fix: each character that has a key on the US layout is sent on that key, with the character attached.
- **The first gate dialog was answered in under four seconds, three times.** It was the person at the Mac, clicking Allow on test dialogs. It still exposed the real flaw: the system alert has keyboard paths. That led to the panel.
- **Browsers answer the first read with no page.** Safari and Chrome build a page's tree only when asked. The first `see` returned the toolbar and nothing else. Fix: ask again when a browser window comes back with no page.
- **A button did not do what its label said.** "Download" sent the tab to the file and may have played it aloud. The staged change survived only because the browser kept the page in memory.
- **A dropdown hides its options.** A closed list shows only its current choice. Each one had to be opened to be read.
- **Icons are blank.** A page that draws icons with a font gives them no name. The glyphs are now dropped from labels, but an icon-only button still has no label.
- **A test page took the focus.** A test window opened in the browser while the person was working. Checks now open things in the background or hidden.
- **The command line forgets.** Each `osctl call` is its own process, so ids from one call are gone in the next. The first job needed a small bridge to hold one session open. A script that scraped an id with grep then sent an empty request and killed the bridge. The listing is written for a model to read, not for a script to parse.
- **A background edit does not count as an edit.** Setting a text area's value through the accessibility API fills it, but TextEdit does not mark the document as changed. A demo that depended on its save prompt had to change.
- **No timing signal.** There is no way to wait for a page to settle, so the first job slept about a second after each press.
- **CI is not on.** The GitHub token could not push a workflow file. The workflow exists locally and is not in the repo yet.

## Verification

- `make test`: 44 green, 0 failing. 36 in `os-core`, 4 in `osctl`, 4 in `os-bar`.
- `python3 tests/live.py`: 24 of 24 on the final build. Calculator in the background. Includes a gated click that waited, got no answer, and was refused.
- `python3 tests/live.py --pointer`: 26 of 26 on an earlier build of the same day. The extra checks send the panel a posted click and an accessibility press, and pass only if both are refused.
- `python3 tests/foreground.py`: 5 of 5 after the key code fix. Real key presses and typed text reach the app in front, and the focus goes back.
- A real Claude Code run: 123 × 45 = 5535 in Calculator in the background, 12 turns, 11 s.
- The bar ran tasks end to end, including the demo run that hits the money gate.
- Fresh clone of the public repo: unit tests pass, and the full release build makes all three binaries in 1 min 13 s.
- Machine: MacBook Air M5, 24 GB, macOS 26.5.

Not checked live: `scroll`, `drag`, right click, double click, position clicks on a second display, the corner stop gesture, the bar's Stop buttons, the money rules on a real bank page, `osctl grant` on a Mac that lacks the grants, and steering a session from a phone.

## Next steps

1. **Turn on CI.** Blocked on a token with the `workflow` scope. Then push `.github/workflows/test.yml` and add the badge.
2. **Say what changed after each act.** The page went elsewhere, a window opened, or nothing changed. This would have caught the button that navigated.
3. **Refuse to act on a window that changed since the last look.** The person may switch tabs between two calls.
4. **Dropdowns in one call.** Read the choices without opening the list, and pick one.
5. **Wait until a page settles.** Replace the fixed sleeps.
6. **Name icon-only buttons** from the classes of the element.
7. **Capture one window even when it is covered.** A full display screenshot is useless when another window is on top.
8. **A session for the command line**, so ids last between `osctl call` runs, and a JSON output for scripts.
9. **Decide on a fifth gate.** Save on a live web service changes something real, and none of the four gates covers it.
10. **Live checks for what is untried:** `scroll`, `drag`, right click, double click, and the corner gesture.
11. **A signed app bundle** with its own grants, so the bar can start from the Dock.
12. **Other languages and other Macs.** The label rules read English. Intel Macs and older systems are untried.

## Pointers

- `README.md`: what it is, the limits, and the same list under Next.
- `crates/os-core/src/gate.rs`: the gate rules and their tests.
- `crates/os-core/src/exec.rs`: every tool.
- `helpers/ask.swift`: the panel, and the check that makes a click count as real.
- `tests/live.py` and `tests/foreground.py`: the live checks.
- `demo/record.py`: how the demo was recorded.
- `~/Library/Application Support/OS Control/audit.jsonl`: what the tools did on this Mac.
