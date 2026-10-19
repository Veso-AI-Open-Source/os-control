//! The first run, as a mockup: connect an AI the person already pays for,
//! allow the two macOS grants, then run a first task.
//!
//! Steps that are already done are skipped. Each screen has one primary
//! action in the bottom right, and Return presses it. A step moves on by
//! itself once its last item turns green. Esc goes back, and the arrow
//! keys move the pick.
//!
//! OS Control never sees a login. Each tool signs in with its own command,
//! in its own Terminal window, and reports back through its own status
//! command. OS Control reads a yes or a no and nothing else.
//!
//!     cargo run -p os-bar --example onboarding
//!
//! ONBOARD_FRESH=1 starts as a Mac with nothing set up, and every button
//! only pretends. Use it to click through the flow with no side effects.
//! For design review, ONBOARD_STEP=grants or try opens on that step, and
//! ONBOARD_THEME=light or dark overrides the system appearance.

#[allow(dead_code)]
#[path = "../src/theme.rs"]
mod theme;
#[allow(dead_code)]
#[path = "../src/ui.rs"]
mod ui;

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Application, Bounds, ClickEvent, Context, FocusHandle, FontWeight,
    KeyDownEvent, Render, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use os_core::{ax, input, screen, state};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use theme::{theme, Theme, BODY, HEADER_BAR, LABEL_SMALL, TOP_BAR_LEADING};
use ui::{Badge, Button, Card, Tone};

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Connect,
    Grants,
    Try,
}

static STEPS: [Step; 3] = [Step::Connect, Step::Grants, Step::Try];

/// The two grants macOS asks the person for.
#[derive(Clone, Copy)]
enum Grant {
    Accessibility,
    Screen,
}

impl Grant {
    const ALL: [Grant; 2] = [Grant::Accessibility, Grant::Screen];

    fn title(self) -> &'static str {
        match self {
            Grant::Accessibility => "Accessibility",
            Grant::Screen => "Screen Recording",
        }
    }

    fn why(self) -> &'static str {
        match self {
            Grant::Accessibility => "Read and press controls in every app.",
            Grant::Screen => "See apps that have no controls to read.",
        }
    }

    fn granted(self) -> bool {
        match self {
            // Posting keys and clicks is listed under Accessibility too.
            Grant::Accessibility => ax::trusted() && input::can_post(),
            Grant::Screen => screen::can_capture(),
        }
    }

    /// Shows the macOS prompt, and the Settings pane when the prompt is not enough.
    fn request(self) {
        let (ok, pane) = match self {
            Grant::Accessibility => (ax::request_trust(), "Privacy_Accessibility"),
            Grant::Screen => (screen::request_capture(), "Privacy_ScreenCapture"),
        };
        if !ok {
            let url = format!("x-apple.systempreferences:com.apple.preference.security?{pane}");
            let _ = Command::new("/usr/bin/open").arg(url).status();
        }
    }
}

/// An AI tool the person may already pay for. OS Control drives the
/// official command, signed in by the person, and never reads its login.
struct Tool {
    name: &'static str,
    bin: &'static str,
    plan: &'static str,
    get_url: &'static str,
    login: &'static [&'static str],
}

static TOOLS: [Tool; 3] = [
    Tool {
        name: "Claude Code",
        bin: "claude",
        plan: "Uses your Claude plan",
        get_url: "https://claude.com/claude-code",
        login: &["auth", "login"],
    },
    Tool {
        name: "Codex",
        bin: "codex",
        plan: "Uses your ChatGPT plan",
        get_url: "https://github.com/openai/codex",
        login: &["login"],
    },
    Tool {
        name: "Gemini CLI",
        bin: "gemini",
        plan: "Uses your Google account",
        get_url: "https://github.com/google-gemini/gemini-cli",
        // Gemini CLI signs in on its first run.
        login: &[],
    },
];

/// First tasks that only read. Name, what it does, and what the bar is told.
static TASKS: [(&str, &str, &str); 3] = [
    (
        "Do a sum in Calculator",
        "Works out 123 × 45, in the background.",
        "Open Calculator in the background and work out 123 × 45. Tell me the answer.",
    ),
    (
        "Check who can record the screen",
        "Reads System Settings. Changes nothing.",
        "Open System Settings and tell me which apps can record the screen. Change nothing.",
    ),
    (
        "Summarise a web page",
        "Reads the front Safari page. Changes nothing.",
        "Read the page in the front Safari window and summarise it in three lines. Change nothing.",
    ),
];

static GATES: [&str; 4] = ["Money", "Sending as you", "Passwords", "Deleting"];

#[derive(Clone, PartialEq)]
enum Login {
    Checking,
    Missing,
    SignedOut,
    /// Signed in, with how, when the tool says.
    SignedIn(Option<String>),
    /// The tool has no status command. It signs in on first use.
    Unknown,
}

/// Where a command lives. An app started from the Dock has no shell PATH,
/// so the usual install places are tried too.
fn find(bin: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut places: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    places.push(home.join(".local/bin"));
    places.push(home.join(".claude/local"));
    places.push("/opt/homebrew/bin".into());
    places.push("/usr/local/bin".into());
    places.into_iter().map(|d| d.join(bin)).find(|p| p.is_file())
}

/// Asks the tool itself whether it is signed in. It starts a process, so
/// run it off the main thread.
fn check(tool: &Tool) -> (Option<PathBuf>, Login) {
    let Some(path) = find(tool.bin) else {
        return (None, Login::Missing);
    };
    let login = match tool.bin {
        // Prints JSON. Two fields are read. The rest, such as the email, is ignored.
        "claude" => Command::new(&path)
            .args(["auth", "status"])
            .stdin(Stdio::null())
            .output()
            .ok()
            .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
            .map(|v| {
                if v["loggedIn"].as_bool() == Some(true) {
                    Login::SignedIn(v["authMethod"].as_str().map(str::to_string))
                } else {
                    Login::SignedOut
                }
            })
            .unwrap_or(Login::SignedOut),
        // Exits 0 when signed in. Check this against the Codex version you ship with.
        "codex" => match Command::new(&path).args(["login", "status"]).stdin(Stdio::null()).output() {
            Ok(o) if o.status.success() => Login::SignedIn(None),
            _ => Login::SignedOut,
        },
        _ => Login::Unknown,
    };
    (Some(path), login)
}

/// Opens Terminal on the tool's own login. A .command file opens there
/// with no Automation grant, and the login never passes through this app.
fn sign_in(tool: &Tool, path: &Path) -> Result<(), String> {
    let quoted = format!("'{}'", path.display().to_string().replace('\'', r"'\''"));
    let script = format!("#!/bin/zsh\n{quoted} {}\n", tool.login.join(" "));
    let file = state::tmp_dir().join(format!("sign-in-{}.command", tool.bin));
    std::fs::write(&file, script).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    Command::new("/usr/bin/open").arg(&file).status().map_err(|e| e.to_string())?;
    Ok(())
}

/// The bar: installed on the PATH, or built next to this example.
fn find_bar() -> Option<PathBuf> {
    find("os-bar").or_else(|| {
        let exe = std::env::current_exe().ok()?;
        Some(exe.parent()?.parent()?.join("os-bar")).filter(|p| p.is_file())
    })
}

/// Steps through 0..len, wrapping at both ends.
fn wrap(i: usize, delta: isize, len: usize) -> usize {
    (i as isize + delta).rem_euclid(len as isize) as usize
}

struct Setup {
    step: Step,
    /// Preview mode: nothing is set up, and buttons have no side effects.
    fresh: bool,
    focus: FocusHandle,
    grants: [bool; 2],
    tools: Vec<(Option<PathBuf>, Login)>,
    /// The picked tool card. The primary action acts on it.
    tool: usize,
    task: usize,
    checking: bool,
    /// Until the first look at this Mac has skipped the steps already done.
    routing: bool,
    /// A sign-in started here, so its success can move on by itself.
    awaiting: Option<usize>,
    /// Bumped on each step change. A pending auto-advance from an older
    /// step sees a different number and does nothing.
    epoch: u64,
    flash: Option<(SharedString, Tone)>,
    ticks: u64,
}

impl Setup {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Grants change in System Settings and logins in Terminal, so look
        // again each second.
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(1)).await;
            if this.update(cx, |s: &mut Self, cx| s.tick(cx)).is_err() {
                break;
            }
        })
        .detach();

        let focus = cx.focus_handle();
        window.focus(&focus);
        let fresh = std::env::var_os("ONBOARD_FRESH").is_some();
        let mut setup = Self {
            step: Step::Connect,
            fresh,
            focus,
            grants: [false; 2],
            tools: TOOLS.iter().map(|_| (None, Login::Checking)).collect(),
            tool: 0,
            task: 0,
            checking: false,
            routing: !fresh,
            awaiting: None,
            epoch: 0,
            flash: None,
            ticks: 0,
        };
        if fresh {
            setup.tools = vec![
                (Some("claude".into()), Login::SignedOut),
                (None, Login::Missing),
                (None, Login::Missing),
            ];
            match std::env::var("ONBOARD_STEP").as_deref() {
                Ok("grants") => {
                    setup.tools[0].1 = Login::SignedIn(Some("claude.ai".into()));
                    setup.step = Step::Grants;
                    setup.grants = [true, false];
                }
                Ok("try") => {
                    setup.tools[0].1 = Login::SignedIn(Some("claude.ai".into()));
                    setup.step = Step::Try;
                    setup.grants = [true, true];
                }
                _ => {}
            }
        } else {
            setup.grants = Grant::ALL.map(|g| g.granted());
            setup.refresh_tools(cx);
        }
        setup
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        self.ticks += 1;
        if !self.fresh {
            let before = self.grants;
            self.grants = Grant::ALL.map(|g| g.granted());
            if before != self.grants {
                self.grants_changed(cx);
            }
            if self.step == Step::Connect && self.ticks % 3 == 0 {
                self.refresh_tools(cx);
            }
        }
        cx.notify();
    }

    fn refresh_tools(&mut self, cx: &mut Context<Self>) {
        if self.checking {
            return;
        }
        self.checking = true;
        cx.spawn(async move |this, cx| {
            let found = cx
                .background_executor()
                .spawn(async { TOOLS.iter().map(check).collect::<Vec<_>>() })
                .await;
            let _ = this.update(cx, |s: &mut Self, cx| {
                s.tools = found;
                s.checking = false;
                s.tools_changed(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// The first signed-in tool, else the first that could be.
    fn default_tool(&self) -> usize {
        let first = |want: fn(&Login) -> bool| self.tools.iter().position(|(_, l)| want(l));
        first(|l| matches!(l, Login::SignedIn(_)))
            .or_else(|| first(|l| *l == Login::SignedOut))
            .or_else(|| first(|l| *l == Login::Unknown))
            .unwrap_or(0)
    }

    fn tools_changed(&mut self, cx: &mut Context<Self>) {
        // A sign-in started here has finished: pick it and move on.
        if let Some(i) = self.awaiting {
            if matches!(self.tools[i].1, Login::SignedIn(_)) {
                self.awaiting = None;
                self.tool = i;
                self.flash = Some((format!("{} connected", TOOLS[i].name).into(), Tone::Success));
                if self.step == Step::Connect {
                    self.advance_soon(cx);
                }
            }
        }
        // The first look at this Mac skips whatever is already done.
        if self.routing {
            self.routing = false;
            self.tool = self.default_tool();
            if matches!(self.tools[self.tool].1, Login::SignedIn(_)) {
                let next = self.after(Step::Connect);
                self.go(next, cx);
                let done = if next == Step::Try { "You're already set up" } else { "AI connected" };
                self.flash = Some((done.into(), Tone::Success));
            }
        }
    }

    fn grants_changed(&mut self, cx: &mut Context<Self>) {
        if self.step == Step::Grants && self.grants.iter().all(|g| *g) {
            self.flash = Some(("Both on".into(), Tone::Success));
            self.advance_soon(cx);
        }
    }

    /// The step after `step`, skipping the grants when both are on.
    fn after(&self, step: Step) -> Step {
        match step {
            Step::Connect if self.grants.iter().all(|g| *g) => Step::Try,
            Step::Connect => Step::Grants,
            Step::Grants | Step::Try => Step::Try,
        }
    }

    fn go(&mut self, step: Step, cx: &mut Context<Self>) {
        self.step = step;
        self.epoch += 1;
        self.flash = None;
        if step == Step::Connect && !self.fresh {
            self.refresh_tools(cx);
        }
        cx.notify();
    }

    /// Moves on after a beat, so the green tick is seen first.
    fn advance_soon(&mut self, cx: &mut Context<Self>) {
        let epoch = self.epoch;
        self.later(700, cx, move |s, cx| {
            if s.epoch == epoch {
                let next = s.after(s.step);
                let flash = s.flash.take();
                s.go(next, cx);
                s.flash = flash;
            }
        });
    }

    /// Runs `f` after a pause. Preview mode uses it to fake what the real
    /// world would do.
    fn later(&mut self, ms: u64, cx: &mut Context<Self>, f: impl FnOnce(&mut Self, &mut Context<Self>) + 'static) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(ms)).await;
            let _ = this.update(cx, |s: &mut Self, cx| {
                f(s, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn allow(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.fresh {
            self.later(1000, cx, move |s, cx| {
                s.grants[i] = true;
                s.grants_changed(cx);
            });
        } else {
            Grant::ALL[i].request();
        }
    }

    fn get_tool(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.fresh {
            self.later(1200, cx, move |s, _| s.tools[i] = (Some(TOOLS[i].bin.into()), Login::SignedOut));
        } else {
            cx.open_url(TOOLS[i].get_url);
        }
        self.flash = Some(("Install it, then come back".into(), Tone::Neutral));
        cx.notify();
    }

    fn sign_in_tool(&mut self, i: usize, cx: &mut Context<Self>) {
        let tool = &TOOLS[i];
        if self.fresh {
            self.later(1800, cx, move |s, cx| {
                s.tools[i].1 = Login::SignedIn((i == 0).then(|| "claude.ai".to_string()));
                s.tools_changed(cx);
            });
        } else if let Some(path) = self.tools[i].0.clone() {
            if let Err(e) = sign_in(tool, &path) {
                self.flash = Some((format!("Couldn't open Terminal: {e}").into(), Tone::Error));
                cx.notify();
                return;
            }
        }
        self.awaiting = Some(i);
        self.flash = Some(("Finish signing in, in Terminal".into(), Tone::Accent));
        cx.notify();
    }

    fn run_task(&mut self, cx: &mut Context<Self>) {
        let prompt = TASKS[self.task].2;
        self.flash = Some(if TOOLS[self.tool].bin != "claude" {
            ("The bar runs Claude Code today".into(), Tone::Warning)
        } else if self.fresh {
            ("Preview: the bar would open and run this".into(), Tone::Accent)
        } else {
            match find_bar() {
                Some(bar) => {
                    let started = Command::new(bar)
                        .env("OSBAR_AUTOSEND", prompt)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .process_group(0)
                        .spawn();
                    match started {
                        Ok(_) => return cx.quit(),
                        Err(e) => (format!("Couldn't open the bar: {e}").into(), Tone::Error),
                    }
                }
                None => ("The bar isn't installed. Run make install".into(), Tone::Error),
            }
        });
        cx.notify();
    }

    /// The first grant still off.
    fn missing_grant(&self) -> Option<usize> {
        self.grants.iter().position(|g| !*g)
    }

    /// What the one primary button says on this screen, and whether it can be pressed.
    fn primary(&self) -> (SharedString, bool) {
        match self.step {
            Step::Connect => {
                let tool = &TOOLS[self.tool];
                match &self.tools[self.tool].1 {
                    Login::Checking => ("Checking…".into(), false),
                    Login::Missing => (format!("Get {}", tool.name).into(), true),
                    Login::SignedOut if self.awaiting == Some(self.tool) => ("Waiting for sign-in…".into(), false),
                    Login::SignedOut => (format!("Sign in to {}", tool.name).into(), true),
                    Login::SignedIn(_) | Login::Unknown => ("Continue".into(), true),
                }
            }
            Step::Grants => match self.missing_grant() {
                Some(i) => (format!("Open {} settings", Grant::ALL[i].title()).into(), true),
                None => ("Continue".into(), true),
            },
            Step::Try => ("Run task".into(), true),
        }
    }

    fn press_primary(&mut self, cx: &mut Context<Self>) {
        if !self.primary().1 {
            return;
        }
        match self.step {
            Step::Connect => match self.tools[self.tool].1 {
                Login::Missing => self.get_tool(self.tool, cx),
                Login::SignedOut => self.sign_in_tool(self.tool, cx),
                _ => self.go(self.after(Step::Connect), cx),
            },
            Step::Grants => match self.missing_grant() {
                Some(i) => self.allow(i, cx),
                None => self.go(Step::Try, cx),
            },
            Step::Try => self.run_task(cx),
        }
    }

    fn back(&mut self, cx: &mut Context<Self>) {
        match self.step {
            Step::Connect => {}
            Step::Grants => self.go(Step::Connect, cx),
            Step::Try => self.go(Step::Grants, cx),
        }
    }

    fn pick(&mut self, delta: isize, cx: &mut Context<Self>) {
        match self.step {
            Step::Connect => self.tool = wrap(self.tool, delta, TOOLS.len()),
            Step::Try => self.task = wrap(self.task, delta, TASKS.len()),
            Step::Grants => return,
        }
        cx.notify();
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match ev.keystroke.key.as_str() {
            "enter" => self.press_primary(cx),
            "escape" => self.back(cx),
            "up" => self.pick(-1, cx),
            "down" => self.pick(1, cx),
            _ => {}
        }
    }

    fn connect_view(&self, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let cards: Vec<_> = TOOLS
            .iter()
            .enumerate()
            .map(|(i, tool)| {
                let login = &self.tools[i].1;
                let (badge, tone) = match login {
                    Login::Checking => ("Checking", Tone::Neutral),
                    Login::Missing => ("Not installed", Tone::Neutral),
                    Login::SignedOut => ("Signed out", Tone::Warning),
                    Login::SignedIn(_) => ("Connected", Tone::Success),
                    Login::Unknown => ("Installed", Tone::Neutral),
                };
                let detail: SharedString = match login {
                    Login::SignedIn(Some(how)) => format!("Signed in with {how}").into(),
                    _ => tool.plan.into(),
                };
                Card::new(("tool", i))
                    .selected(i == self.tool)
                    .on_click(cx.listener(move |s, _: &ClickEvent, _, cx| {
                        s.tool = i;
                        cx.notify();
                    }))
                    .child(ui::radio(t, i == self.tool))
                    .child(ui::item_text(t, tool.name, detail))
                    .child(Badge::new(badge, tone))
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(ui::headline(t, "Connect your AI", "OS Control gives the AI you already pay for hands on this Mac."))
            .child(div().flex().flex_col().gap_2().children(cards))
            .child(ui::note(t, "Your login stays with the tool. OS Control never sees it."))
    }

    fn grants_view(&self, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.missing_grant();
        let cards: Vec<_> = Grant::ALL
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let (badge, tone) = if self.grants[i] {
                    ("On", Tone::Success)
                } else if current == Some(i) {
                    ("Needed", Tone::Warning)
                } else {
                    ("Next", Tone::Neutral)
                };
                Card::new(("grant", i))
                    .selected(current == Some(i))
                    .when(!self.grants[i], |card| {
                        card.on_click(cx.listener(move |s, _: &ClickEvent, _, cx| s.allow(i, cx)))
                    })
                    .child(ui::item_text(t, g.title(), g.why()))
                    .child(Badge::new(badge, tone))
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(ui::headline(t, "Allow two permissions", "macOS asks before any app can use another."))
            .child(div().flex().flex_col().gap_2().children(cards))
            .child(ui::note(t, "Turn them off any time in System Settings › Privacy & Security."))
    }

    fn try_view(&self, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let cards: Vec<_> = TASKS
            .iter()
            .enumerate()
            .map(|(i, (name, detail, _))| {
                Card::new(("task", i))
                    .selected(i == self.task)
                    .on_click(cx.listener(move |s, _: &ClickEvent, _, cx| {
                        s.task = i;
                        cx.notify();
                    }))
                    .child(ui::radio(t, i == self.task))
                    .child(ui::item_text(t, *name, *detail))
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(ui::headline(
                t,
                "Try it",
                format!("{} works in the background. Keep working.", TOOLS[self.tool].name),
            ))
            .child(div().flex().flex_col().gap_2().children(cards))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(ui::section_label(t, "ALWAYS ASKS YOU FIRST"))
                    .child(div().flex().flex_wrap().gap_2().children(GATES.map(|g| Badge::new(g, Tone::Neutral))))
                    .child(ui::note(t, "To stop everything, push the pointer into the top-left corner.")),
            )
    }
}

impl Render for Setup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *theme(cx);
        let at = STEPS.iter().position(|s| *s == self.step).unwrap_or(0);
        let page = match self.step {
            Step::Connect => self.connect_view(&t, cx).into_any_element(),
            Step::Grants => self.grants_view(&t, cx).into_any_element(),
            Step::Try => self.try_view(&t, cx).into_any_element(),
        };
        let (label, enabled) = self.primary();
        let lists = matches!(self.step, Step::Connect | Step::Try);

        let header = div()
            .h(HEADER_BAR)
            .flex_none()
            .pl(TOP_BAR_LEADING)
            .pr_4()
            .border_b_1()
            .border_color(t.border)
            .flex()
            .items_center()
            .justify_between()
            .child(div().text_size(LABEL_SMALL).font_weight(FontWeight::BOLD).child("OS CONTROL"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(ui::steps(&t, STEPS.len(), at))
                    .child(
                        div()
                            .text_size(LABEL_SMALL)
                            .text_color(t.text_faint)
                            .child(SharedString::from(format!("{} of {}", at + 1, STEPS.len()))),
                    ),
            );

        let footer = div()
            .flex_none()
            .h(px(52.))
            .px_4()
            .border_t_1()
            .border_color(t.border)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .when(at > 0, |el| {
                        el.child(
                            Button::new("back", "Back")
                                .ghost()
                                .key("esc")
                                .on_click(cx.listener(|s, _: &ClickEvent, _, cx| s.back(cx))),
                        )
                    })
                    .map(|el| match self.flash.clone() {
                        Some((text, tone)) => el.child(ui::arrive(("flash", self.epoch), div().child(Badge::new(text, tone)))),
                        None if lists => el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_size(LABEL_SMALL)
                                .text_color(t.text_faint)
                                .child(ui::kbd(&t, "↑↓"))
                                .child("to choose"),
                        ),
                        None => el,
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when(self.step == Step::Try, |el| {
                        el.child(
                            Button::new("skip", "Skip")
                                .ghost()
                                .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.quit())),
                        )
                    })
                    .child(
                        Button::new("primary", label)
                            .primary()
                            .key("↩")
                            .disabled(!enabled)
                            .on_click(cx.listener(|s, _: &ClickEvent, _, cx| s.press_primary(cx))),
                    ),
            );

        div()
            .id("setup")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .size_full()
            .bg(t.surface)
            .text_color(t.text)
            .text_size(BODY)
            .flex()
            .flex_col()
            .child(header)
            .child(
                div()
                    .id("body")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_6()
                    .pt_6()
                    .pb_6()
                    .flex()
                    .justify_center()
                    .child(ui::arrive(("page", self.epoch), div().w_full().max_w(px(440.)).child(page))),
            )
            .child(footer)
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.set_global(Theme::for_appearance(cx.window_appearance()));
        let bounds = Bounds::centered(None, size(px(600.), px(540.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("OS Control Setup".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(gpui::point(px(14.), px(14.))),
                }),
                window_min_size: Some(size(px(540.), px(500.))),
                ..Default::default()
            },
            |window, cx| {
                match std::env::var("ONBOARD_THEME").as_deref() {
                    Ok("light") => cx.set_global(Theme::light()),
                    Ok("dark") => cx.set_global(Theme::dark()),
                    _ => theme::follow_system(window, cx),
                }
                cx.new(|cx| Setup::new(window, cx))
            },
        )
        .unwrap();
        cx.activate(true);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
    });
}
