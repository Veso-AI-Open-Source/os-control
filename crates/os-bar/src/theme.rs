//! The design tokens every OS Control window draws with.
//!
//! One rule: chrome reads a slot and never a raw hex value. A few values
//! are stated per appearance, and the rest are blended from them once, here.
//!
//! Scale, so screens stay consistent:
//! - Space: a 4 px grid. Use the 1, 2, 3, 4, 6 and 8 steps (4 to 32 px).
//! - Type: DISPLAY for the one headline on a screen, TITLE for an item's
//!   name, BODY for prose, BODY_SMALL for secondary lines, LABEL_SMALL for
//!   badges, keys and buttons.
//! - Radius: RADIUS_CONTROL for anything you press, RADIUS_CARD for groups.
//! - Color: accent only on the one primary action, the current step, and
//!   the selection. Status always pairs a color with a glyph or a word.

use gpui::{px, rgb, App, Global, Pixels, Rgba, Window, WindowAppearance};
use std::time::Duration;

/// Blend `fg` at `alpha` over `bg`, resolving to a solid color.
fn over(bg: Rgba, fg: Rgba, alpha: f32) -> Rgba {
    bg.blend(Rgba { a: alpha, ..fg })
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub background: Rgba,
    pub surface: Rgba,
    pub elevated_surface: Rgba,
    pub element_hover: Rgba,
    pub border: Rgba,
    pub border_strong: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    /// Hints and keys only. Never the only carrier of meaning.
    pub text_faint: Rgba,
    pub accent: Rgba,
    pub accent_hover: Rgba,
    pub accent_subtle: Rgba,
    /// The fill of a picked card. Lighter than a badge, so text stays crisp.
    pub selection: Rgba,
    pub text_on_accent: Rgba,
    pub success: Rgba,
    pub success_subtle: Rgba,
    pub warning: Rgba,
    pub warning_subtle: Rgba,
    pub error: Rgba,
    pub error_subtle: Rgba,
    pub neutral_subtle: Rgba,
}

impl Theme {
    #[allow(clippy::too_many_arguments)]
    fn build(
        background: Rgba,
        surface: Rgba,
        elevated_surface: Rgba,
        border: Rgba,
        border_strong: Rgba,
        text: Rgba,
        accent: Rgba,
        text_on_accent: Rgba,
        status: [Rgba; 3],
        tint: f32,
    ) -> Self {
        let [success, warning, error] = status;
        Self {
            background,
            surface,
            elevated_surface,
            element_hover: over(elevated_surface, text, 0.06),
            border,
            border_strong,
            text,
            text_muted: over(background, text, 0.64),
            text_faint: over(background, text, 0.42),
            accent,
            accent_hover: over(accent, text_on_accent, 0.12),
            accent_subtle: over(surface, accent, tint),
            selection: over(elevated_surface, accent, tint * 0.5),
            text_on_accent,
            success,
            success_subtle: over(surface, success, tint),
            warning,
            warning_subtle: over(surface, warning, tint),
            error,
            error_subtle: over(surface, error, tint),
            neutral_subtle: over(surface, text, 0.07),
        }
    }

    pub fn dark() -> Self {
        Self::build(
            rgb(0x0A0A0B),
            rgb(0x111113),
            rgb(0x18181B),
            rgb(0x27272A),
            rgb(0x3F3F46),
            rgb(0xEDEDEF),
            rgb(0x9B6DFF),
            rgb(0x0A0A0B),
            [rgb(0x4CC38A), rgb(0xF5A524), rgb(0xF0616D)],
            0.14,
        )
    }

    pub fn light() -> Self {
        Self::build(
            rgb(0xFFFFFF),
            rgb(0xF6F6F7),
            rgb(0xFFFFFF),
            rgb(0xE4E4E7),
            rgb(0xC9C9CF),
            rgb(0x18181B),
            rgb(0x7C4DFF),
            rgb(0xFFFFFF),
            [rgb(0x15803D), rgb(0xB45309), rgb(0xDC2626)],
            0.10,
        )
    }

    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
        }
    }
}

impl Global for Theme {}

pub fn theme(cx: &App) -> &Theme {
    cx.global::<Theme>()
}

/// Matches the window to macOS now, and again whenever the person switches
/// between light and dark.
pub fn follow_system(window: &mut Window, cx: &mut App) {
    cx.set_global(Theme::for_appearance(window.appearance()));
    window
        .observe_window_appearance(|window, cx| {
            cx.set_global(Theme::for_appearance(window.appearance()));
            window.refresh();
        })
        .detach();
}

/// The height of the top bar.
pub const HEADER_BAR: Pixels = px(40.);
/// Where content starts, clear of the macOS traffic lights.
pub const TOP_BAR_LEADING: Pixels = px(84.);
/// Interactive controls: buttons, chips, inputs.
pub const RADIUS_CONTROL: Pixels = px(4.);
/// Cards and grouped rows.
pub const RADIUS_CARD: Pixels = px(6.);
/// The height of a button.
pub const CONTROL_HEIGHT: Pixels = px(28.);

pub const DISPLAY: Pixels = px(22.);
pub const TITLE: Pixels = px(13.);
pub const BODY: Pixels = px(13.);
pub const BODY_SMALL: Pixels = px(12.);
pub const LABEL_SMALL: Pixels = px(11.);

/// A screen or a section arriving.
pub const MOTION_BASE: Duration = Duration::from_millis(220);
