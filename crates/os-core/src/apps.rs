//! Running apps: which one is in front, and how to open, raise, and quit them.

use crate::ax::{self, Element};
use crate::ffi::*;
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
use std::process::Command;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppInfo {
    pub pid: i32,
    pub name: String,
    pub bundle: String,
}

type Id = *mut c_void;

unsafe fn sel(name: &CStr) -> Id {
    sel_registerName(name.as_ptr())
}

unsafe fn msg_ptr() -> *const () {
    objc_msgSend as *const ()
}

unsafe fn send_id(obj: Id, s: Id) -> Id {
    let f: extern "C" fn(Id, Id) -> Id = std::mem::transmute(msg_ptr());
    f(obj, s)
}

unsafe fn send_bool(obj: Id, s: Id) -> bool {
    let f: extern "C" fn(Id, Id) -> bool = std::mem::transmute(msg_ptr());
    f(obj, s)
}

unsafe fn running_app(pid: i32) -> Id {
    let cls = objc_getClass(c"NSRunningApplication".as_ptr());
    if cls.is_null() {
        return std::ptr::null_mut();
    }
    let f: extern "C" fn(Id, Id, i32) -> Id = std::mem::transmute(msg_ptr());
    f(cls, sel(c"runningApplicationWithProcessIdentifier:"), pid)
}

unsafe fn ns_string(obj: Id) -> String {
    if obj.is_null() {
        return String::new();
    }
    let f: extern "C" fn(Id, Id) -> *const c_char = std::mem::transmute(msg_ptr());
    let p = f(obj, sel(c"UTF8String"));
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

/// Runs `f` with the app object for a pid inside an autorelease pool.
fn with_app<T>(pid: i32, f: impl FnOnce(Id) -> T) -> Option<T> {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let app = running_app(pid);
        let out = if app.is_null() { None } else { Some(f(app)) };
        objc_autoreleasePoolPop(pool);
        out
    }
}

/// Name and bundle id for a pid. Empty strings when the pid is not an app.
pub fn info(pid: i32) -> AppInfo {
    with_app(pid, |app| unsafe {
        AppInfo {
            pid,
            name: ns_string(send_id(app, sel(c"localizedName"))),
            bundle: ns_string(send_id(app, sel(c"bundleIdentifier"))),
        }
    })
    .unwrap_or(AppInfo { pid, ..Default::default() })
}

fn lsappinfo(args: &[&str]) -> String {
    Command::new("/usr/bin/lsappinfo")
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// The app that has keyboard focus right now. Asked fresh on every call.
pub fn frontmost() -> Option<AppInfo> {
    if let Some((_, pid)) = ax::focused_app() {
        let i = info(pid);
        if !i.name.is_empty() {
            return Some(i);
        }
    }
    let asn = lsappinfo(&["front"]);
    let asn = asn.trim();
    if asn.is_empty() {
        return None;
    }
    let text = lsappinfo(&["info", "-only", "pid", asn]);
    let pid: i32 = text.split('=').nth(1)?.trim().parse().ok()?;
    Some(info(pid))
}

/// Parses `lsappinfo list` into the apps that show in the Dock.
pub fn parse_list(text: &str) -> Vec<AppInfo> {
    parse_all(text).into_iter().filter(|(_, dock)| *dock).map(|(a, _)| a).collect()
}

/// Every process the system lists, with a flag for apps that show in the
/// Dock. The rest are agents and system dialogs.
pub fn parse_all(text: &str) -> Vec<(AppInfo, bool)> {
    let mut out = Vec::new();
    let mut cur: Option<AppInfo> = None;
    let mut foreground = false;
    let quoted = |s: &str| s.split('"').nth(1).unwrap_or("").to_string();
    for line in text.lines() {
        let t = line.trim_start();
        let is_header = t.split(')').next().map(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())).unwrap_or(false)
            && t.contains("ASN:");
        if is_header {
            if let Some(a) = cur.take() {
                if a.pid > 0 {
                    out.push((a, foreground));
                }
            }
            foreground = false;
            cur = Some(AppInfo { pid: 0, name: quoted(t), bundle: String::new() });
        } else if let Some(a) = cur.as_mut() {
            if t.starts_with("bundleID=") {
                a.bundle = quoted(t);
            } else if t.starts_with("pid = ") {
                a.pid = t[6..].split_whitespace().next().and_then(|p| p.parse().ok()).unwrap_or(0);
                foreground = t.contains("type=\"Foreground\"");
            }
        }
    }
    if let Some(a) = cur {
        if a.pid > 0 {
            out.push((a, foreground));
        }
    }
    out
}

pub fn running() -> Vec<AppInfo> {
    parse_list(&lsappinfo(&["list"]))
}

/// Picks the running apps a query means: bundle id, exact name, then part of
/// a name. The best kind of match wins and the rest are dropped.
pub fn pick(list: &[AppInfo], query: &str) -> Vec<AppInfo> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let by = |f: &dyn Fn(&AppInfo) -> bool| -> Vec<AppInfo> {
        list.iter().filter(|a| f(a)).cloned().collect()
    };
    let exact_bundle = by(&|a| a.bundle.to_lowercase() == q);
    if !exact_bundle.is_empty() {
        return exact_bundle;
    }
    let exact_name = by(&|a| a.name.to_lowercase() == q);
    if !exact_name.is_empty() {
        return exact_name;
    }
    by(&|a| a.name.to_lowercase().contains(&q))
}

/// Running apps a query names. Dock apps win. A system dialog or an agent
/// is found only by its exact name or bundle id.
pub fn find(query: &str) -> Vec<AppInfo> {
    let all = parse_all(&lsappinfo(&["list"]));
    let dock: Vec<AppInfo> = all.iter().filter(|(_, d)| *d).map(|(a, _)| a.clone()).collect();
    let found = pick(&dock, query);
    if !found.is_empty() {
        return found;
    }
    let q = query.trim().to_lowercase();
    all.into_iter()
        .filter(|(a, d)| !*d && (a.name.to_lowercase() == q || a.bundle.to_lowercase() == q))
        .map(|(a, _)| a)
        .collect()
}

/// True when `app` is what `query` names.
pub fn matches(app: &AppInfo, query: &str) -> bool {
    !pick(std::slice::from_ref(app), query).is_empty()
}

fn looks_like_bundle_id(s: &str) -> bool {
    s.matches('.').count() >= 2 && !s.contains(' ') && !s.contains('/')
}

/// Opens an app by name or bundle id, with an optional file or URL.
pub fn open(app: Option<&str>, target: Option<&str>, background: bool) -> Result<(), String> {
    let mut cmd = Command::new("/usr/bin/open");
    if background {
        cmd.arg("-g");
    }
    if let Some(a) = app {
        cmd.arg(if looks_like_bundle_id(a) { "-b" } else { "-a" }).arg(a);
    }
    if let Some(t) = target {
        cmd.arg(t);
    }
    if app.is_none() && target.is_none() {
        return Err("give a name, a bundle, or a target to open".into());
    }
    let out = cmd.output().map_err(|e| format!("could not run open: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Brings an app to the front.
pub fn activate(app: &AppInfo) -> Result<(), String> {
    let el = Element::app(app.pid);
    if el.set_flag("AXFrontmost", true).is_ok() {
        return Ok(());
    }
    let key = if app.bundle.is_empty() { app.name.as_str() } else { app.bundle.as_str() };
    open(Some(key), None, false)
}

/// Asks an app to quit the polite way. It may ask the person to save.
pub fn quit(pid: i32) -> bool {
    with_app(pid, |app| unsafe { send_bool(app, sel(c"terminate")) }).unwrap_or(false)
}

pub fn hide(pid: i32) -> bool {
    with_app(pid, |app| unsafe { send_bool(app, sel(c"hide")) }).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#" 1) "loginwindow" ASN:0x0-0x3003:
    bundleID="com.apple.loginwindow"
    bundle path="/System/Library/CoreServices/loginwindow.app"
    pid = 422 type="UIElement" flavor=3 Version="3085.5.3" fileType="APPL" creator="lgnw" Arch=ARM64
 2) "universalaccessd" ASN:0x0-0x9009:
    bundleID=[ NULL ]
    pid = 659 !signalled type="BackgroundOnly" flavor=3 Version=[ NULL ]  fileType="????"
 3) "Finder" ASN:0x0-0x10010:
    bundleID="com.apple.finder"
    pid = 672 type="Foreground" flavor=3 Version="26.5" fileType="FNDR" creator="MACS" Arch=ARM64
 4) "Google Chrome" ASN:0x0-0x44044:
    bundleID="com.google.Chrome"
    pid = 901 type="Foreground" flavor=3 Version="141" fileType="APPL" Arch=ARM64
"#;

    #[test]
    fn list_keeps_only_dock_apps() {
        let apps = parse_list(SAMPLE);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0], AppInfo { pid: 672, name: "Finder".into(), bundle: "com.apple.finder".into() });
        assert_eq!(apps[1].name, "Google Chrome");
    }

    #[test]
    fn pick_prefers_the_tightest_match() {
        let apps = vec![
            AppInfo { pid: 1, name: "Code".into(), bundle: "com.microsoft.VSCode".into() },
            AppInfo { pid: 2, name: "Xcode".into(), bundle: "com.apple.dt.Xcode".into() },
        ];
        assert_eq!(pick(&apps, "code").len(), 1);
        assert_eq!(pick(&apps, "code")[0].pid, 1);
        assert_eq!(pick(&apps, "com.apple.dt.xcode")[0].pid, 2);
        assert_eq!(pick(&apps, "cod").len(), 2);
        assert!(pick(&apps, "safari").is_empty());
    }

    #[test]
    fn bundle_ids_are_told_from_names() {
        assert!(looks_like_bundle_id("com.apple.calculator"));
        assert!(!looks_like_bundle_id("Google Chrome"));
        assert!(!looks_like_bundle_id("agent.exe"));
    }
}
