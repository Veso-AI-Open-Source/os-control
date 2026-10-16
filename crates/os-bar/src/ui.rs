//! The parts every OS Control window is built from.
//!
//! Each part reads the theme and nothing else, so a window built from them
//! looks like the rest. Parts take their behaviour from the caller: a click
//! handler, a state. None of them holds state of its own.

use crate::theme::{
    theme, Theme, BODY, BODY_SMALL, CONTROL_HEIGHT, DISPLAY, LABEL_SMALL, MOTION_BASE, RADIUS_CARD,
    RADIUS_CONTROL, TITLE,
};
use gpui::prelude::*;
use gpui::{
    div, ease_out_quint, px, Animation, AnimationElement, AnimationExt, AnyElement, App, ClickEvent, Div,
    ElementId, FontWeight, Rgba, SharedString, Window,
};

type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

fn clear() -> Rgba {
    Rgba { r: 0., g: 0., b: 0., a: 0. }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonKind {
    /// The one action a screen is for. At most one per screen.
    Primary,
    Secondary,
    /// Ways out: back, skip, cancel.
    Ghost,
}

/// A pressable label, with an optional key that also presses it.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    kind: ButtonKind,
    disabled: bool,
    key: Option<&'static str>,
    on_click: Option<OnClick>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: ButtonKind::Secondary,
            disabled: false,
            key: None,
            on_click: None,
        }
    }

    pub fn primary(mut self) -> Self {
        self.kind = ButtonKind::Primary;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.kind = ButtonKind::Ghost;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Shows the key inside the button. The window still has to handle it.
    pub fn key(mut self, key: &'static str) -> Self {
        self.key = Some(key);
        self
    }

    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *theme(cx);
        let Button { id, label, kind, disabled, key, on_click } = self;
        let (bg, fg, border, hover) = match kind {
            ButtonKind::Primary => (t.accent, t.text_on_accent, t.accent, t.accent_hover),
            ButtonKind::Secondary => (t.elevated_surface, t.text, t.border_strong, t.element_hover),
            ButtonKind::Ghost => (clear(), t.text_muted, clear(), t.element_hover),
        };
        div()
            .id(id)
            .flex_none()
            .h(CONTROL_HEIGHT)
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .rounded(RADIUS_CONTROL)
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(fg)
            .text_size(BODY_SMALL)
            .font_weight(FontWeight::MEDIUM)
            .child(label)
            .when_some(key, |el, key| el.child(div().text_size(LABEL_SMALL).opacity(0.6).child(key)))
            .map(|el| {
                if disabled {
                    el.opacity(0.4)
                } else {
                    el.cursor_pointer()
                        .hover(move |s| s.bg(hover))
                        .active(|s| s.opacity(0.8))
                        .when_some(on_click, |el, f| el.on_click(f))
                }
            })
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tone {
    Neutral,
    Accent,
    Success,
    Warning,
    Error,
}

impl Tone {
    fn colors(self, t: &Theme) -> (Rgba, Rgba) {
        match self {
            Tone::Neutral => (t.neutral_subtle, t.text_muted),
            Tone::Accent => (t.accent_subtle, t.accent),
            Tone::Success => (t.success_subtle, t.success),
            Tone::Warning => (t.warning_subtle, t.warning),
            Tone::Error => (t.error_subtle, t.error),
        }
    }

    /// Carries the meaning when color cannot.
    fn glyph(self) -> Option<&'static str> {
        match self {
            Tone::Neutral => None,
            Tone::Accent => Some("›"),
            Tone::Success => Some("✓"),
            Tone::Warning => Some("!"),
            Tone::Error => Some("×"),
        }
    }
}

/// A short status: a glyph and a word or two, tinted by tone.
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    tone: Tone,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>, tone: Tone) -> Self {
        Self { label: label.into(), tone }
    }
}

impl RenderOnce for Badge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (bg, fg) = self.tone.colors(theme(cx));
        div()
            .flex_none()
            .h(px(20.))
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .rounded(RADIUS_CONTROL)
            .bg(bg)
            .text_color(fg)
            .text_size(LABEL_SMALL)
            .font_weight(FontWeight::MEDIUM)
            .when_some(self.tone.glyph(), |el, g| el.child(g))
            .child(self.label)
    }
}

/// A row in a list. With a click handler it can be picked, and the picked
/// one shows an accent edge.
#[derive(IntoElement)]
pub struct Card {
    id: ElementId,
    selected: bool,
    on_click: Option<OnClick>,
    children: Vec<AnyElement>,
}

impl Card {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self { id: id.into(), selected: false, on_click: None, children: Vec::new() }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_click(mut self, f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl RenderOnce for Card {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = *theme(cx);
        let selected = self.selected;
        div()
            .id(self.id)
            .w_full()
            .px_3()
            .py(px(10.))
            .flex()
            .items_center()
            .gap_3()
            .rounded(RADIUS_CARD)
            .border_1()
            .border_color(if selected { t.accent } else { t.border })
            .bg(if selected { t.selection } else { t.elevated_surface })
            .when_some(self.on_click, |el, f| {
                el.cursor_pointer()
                    .when(!selected, |el| el.hover(move |s| s.border_color(t.border_strong)))
                    .on_click(f)
            })
            .children(self.children)
    }
}

/// The mark on a card you can pick.
pub fn radio(t: &Theme, on: bool) -> Div {
    div()
        .flex_none()
        .w(px(14.))
        .h(px(14.))
        .rounded_full()
        .border_1()
        .border_color(if on { t.accent } else { t.border_strong })
        .flex()
        .items_center()
        .justify_center()
        .when(on, |el| el.child(div().w(px(6.)).h(px(6.)).rounded_full().bg(t.accent)))
}

/// An item's name, with a quieter line under it.
pub fn item_text(t: &Theme, name: impl Into<SharedString>, detail: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(2.))
        .child(div().text_size(TITLE).font_weight(FontWeight::MEDIUM).text_color(t.text).child(name.into()))
        .child(div().text_size(BODY_SMALL).text_color(t.text_muted).child(detail.into()))
}

/// The one headline on a screen, with one line under it.
pub fn headline(t: &Theme, title: impl Into<SharedString>, sub: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_size(DISPLAY).font_weight(FontWeight::SEMIBOLD).text_color(t.text).child(title.into()))
        .child(div().text_size(BODY).text_color(t.text_muted).child(sub.into()))
}

/// A small label over a group. Pass it in capitals.
pub fn section_label(t: &Theme, text: &'static str) -> Div {
    div().text_size(LABEL_SMALL).font_weight(FontWeight::SEMIBOLD).text_color(t.text_faint).child(text)
}

/// A quiet line under a list.
pub fn note(t: &Theme, text: impl Into<SharedString>) -> Div {
    div().text_size(BODY_SMALL).text_color(t.text_muted).child(text.into())
}

/// A key, shown as a key: ↩, esc, ↑↓.
pub fn kbd(t: &Theme, key: &'static str) -> Div {
    div()
        .flex_none()
        .h(px(18.))
        .px_1()
        .flex()
        .items_center()
        .rounded(px(3.))
        .border_1()
        .border_color(t.border)
        .text_size(LABEL_SMALL)
        .text_color(t.text_faint)
        .child(key)
}

/// Progress through a short flow: one segment per step, filled up to `current`.
pub fn steps(t: &Theme, total: usize, current: usize) -> Div {
    div().flex().items_center().gap_1().children((0..total).map(|i| {
        div()
            .w(px(18.))
            .h(px(3.))
            .rounded(px(2.))
            .bg(if i <= current { t.accent } else { t.border_strong })
    }))
}

/// Fades content in and lifts it into place. A new id plays it again.
pub fn arrive(id: impl Into<ElementId>, el: Div) -> AnimationElement<Div> {
    el.relative().with_animation(
        id,
        Animation::new(MOTION_BASE).with_easing(ease_out_quint()),
        |el, delta| el.opacity(delta).top(px((1. - delta) * 8.)),
    )
}
