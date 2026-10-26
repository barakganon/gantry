//! The custom color picker (an overlay): a saturation/value square, a hue bar and a hex field, for a pane's color or a
//! project's color. It only reports the pick; the window applies it.

use app_core::color_math::{hsv_to_rgb, rgb_to_hsv};
use app_core::menus::ColorTarget;
use app_core::text_field::TextField;
use app_core::theme_colors::{Tint, parse_hex};
use gpui::{
    App, ClickEvent, Context, DispatchPhase, Div, EventEmitter, FocusHandle, Focusable, FontWeight, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, SharedString, Stateful, Window,
    canvas, div, linear_color_stop, linear_gradient, prelude::*, px, relative, rgb, rgba,
};

use crate::field;
use crate::settings_store::SettingsStore;
use crate::theme::{FadeKind, RADIUS_LARGE, RADIUS_SMALL, Theme, fade_in, theme};

/// What the picker asks the window to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ColorPickerEvent {
    /// A color was applied for the target.
    Picked(ColorTarget, Tint),
    /// The picker was dismissed.
    Cancelled,
}

/// Which pad a drag started on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pad {
    Square,
    Hue,
}

/// The picker for one target.
pub(crate) struct ColorPickerView {
    focus_handle: FocusHandle,
    target: ColorTarget,
    hue: f32,
    saturation: f32,
    value: f32,
    hex: TextField,
    dragging: Option<Pad>,
}

impl EventEmitter<ColorPickerEvent> for ColorPickerView {}

impl Focusable for ColorPickerView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl ColorPickerView {
    /// A picker for `target`, starting at `initial` (`0xRRGGBB`).
    pub(crate) fn new(target: ColorTarget, initial: u32, cx: &mut Context<Self>) -> Self {
        let (hue, saturation, value) = rgb_to_hsv(initial);
        let mut hex = TextField::default();
        hex.insert(&format!("#{initial:06x}"));
        ColorPickerView { focus_handle: cx.focus_handle(), target, hue, saturation, value, hex, dragging: None }
    }

    /// The color being picked.
    fn rgb(&self) -> u32 {
        hsv_to_rgb(self.hue, self.saturation, self.value)
    }

    /// Whether the hex field holds a color.
    fn hex_value(&self) -> Option<u32> {
        parse_hex(self.hex.text())
    }

    /// Write the picked color into the hex field.
    fn show_hex(&mut self) {
        self.hex.clear();
        self.hex.insert(&format!("#{:06x}", self.rgb()));
    }

    /// The pointer is at (`x`, `y`), as fractions of the pad it is on.
    fn drag_to(&mut self, pad: Pad, x: f32, y: f32) {
        let (x, y) = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
        match pad {
            Pad::Square => {
                self.saturation = x;
                self.value = 1.0 - y;
            }
            Pad::Hue => self.hue = x * 360.0,
        }
        self.show_hex();
    }

    fn apply(&mut self, cx: &mut Context<Self>) {
        if let Some(rgb) = self.hex_value() {
            cx.emit(ColorPickerEvent::Picked(self.target.clone(), Tint::Custom(rgb)));
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        match keystroke.key.as_str() {
            "escape" => cx.emit(ColorPickerEvent::Cancelled),
            "enter" => self.apply(cx),
            _ => {
                if !field::edit(&mut self.hex, keystroke, cx) {
                    return;
                }
                if let Some(rgb) = self.hex_value() {
                    (self.hue, self.saturation, self.value) = rgb_to_hsv(rgb);
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    /// A pad's mouse handling: press starts a drag, moves follow it, release ends it.
    fn pad(&self, which: Pad, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        canvas(
            |bounds, _, _| bounds,
            move |bounds, _, window, _| {
                let fractions = move |position: gpui::Point<gpui::Pixels>| {
                    ((position.x - bounds.left()) / bounds.size.width, (position.y - bounds.top()) / bounds.size.height)
                };
                let down = entity.clone();
                window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
                    if phase != DispatchPhase::Bubble
                        || event.button != MouseButton::Left
                        || !bounds.contains(&event.position)
                    {
                        return;
                    }
                    let (x, y) = fractions(event.position);
                    down.update(cx, |this, cx| {
                        this.dragging = Some(which);
                        this.drag_to(which, x, y);
                        cx.notify();
                    })
                    .ok();
                });
                let moved = entity.clone();
                window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    let (x, y) = fractions(event.position);
                    moved
                        .update(cx, |this, cx| {
                            if this.dragging == Some(which) {
                                this.drag_to(which, x, y);
                                cx.notify();
                            }
                        })
                        .ok();
                });
                let up = entity.clone();
                window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                    if phase == DispatchPhase::Bubble {
                        up.update(cx, |this, _| this.dragging = None).ok();
                    }
                });
            },
        )
        .absolute()
        .size_full()
    }
}

fn button(id: &'static str, label: impl Into<SharedString>, primary: bool, theme: &Theme) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .h(theme.ui(32.))
        .px(theme.ui(14.))
        .flex()
        .items_center()
        .rounded(px(RADIUS_SMALL))
        .border_1()
        .border_color(rgb(if primary { theme.tokens.accent } else { theme.tokens.line }))
        .when(primary, |b| {
            b.bg(rgb(theme.tokens.accent)).text_color(rgb(theme.tokens.on_accent)).font_weight(FontWeight::SEMIBOLD)
        })
        .when(!primary, |b| b.bg(rgb(theme.tokens.raised)))
        .cursor_pointer()
        .hover(|style| style.border_color(rgb(theme.tokens.accent)))
        .child(label.into())
}

impl Render for ColorPickerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme(cx).clone();
        let focused = self.focus_handle.is_focused(window);
        let valid = self.hex_value().is_some();
        let hue_color = hsv_to_rgb(self.hue, 1.0, 1.0);
        let title = match self.target {
            ColorTarget::Pane(_) => "Tab color",
            ColorTarget::Project(_) => "Project color",
        };
        let marker = |fx: f32, fy: f32| {
            div()
                .absolute()
                .left(relative(fx))
                .top(relative(fy))
                .ml(px(-6.))
                .mt(px(-6.))
                .size(px(12.))
                .rounded_full()
                .border_2()
                .border_color(rgb(0xffffff))
        };
        let square = div()
            .relative()
            .h(theme.ui(140.))
            .rounded(px(RADIUS_SMALL))
            .overflow_hidden()
            .bg(linear_gradient(90., linear_color_stop(rgb(0xffffff), 0.), linear_color_stop(rgb(hue_color), 1.)))
            .child(div().absolute().size_full().bg(linear_gradient(
                180.,
                linear_color_stop(rgba(0x0000_0000), 0.),
                linear_color_stop(rgb(0x000000), 1.),
            )))
            .child(marker(self.saturation, 1.0 - self.value))
            .child(self.pad(Pad::Square, cx));
        let stops = [0xff0000, 0xffff00, 0x00ff00, 0x00ffff, 0x0000ff, 0xff00ff, 0xff0000];
        let bar = div()
            .relative()
            .h(theme.ui(12.))
            .rounded(px(6.))
            .overflow_hidden()
            .flex()
            .children(stops.windows(2).map(|pair| {
                div().flex_1().h_full().bg(linear_gradient(
                    90.,
                    linear_color_stop(rgb(pair[0]), 0.),
                    linear_color_stop(rgb(pair[1]), 1.),
                ))
            }))
            .child(marker(self.hue / 360.0, 0.5))
            .child(self.pad(Pad::Hue, cx));
        let hex = div()
            .flex_1()
            .h(theme.ui(32.))
            .px(theme.ui(10.))
            .flex()
            .items_center()
            .rounded(px(RADIUS_SMALL))
            .border_1()
            .border_color(rgb(if valid { theme.tokens.accent } else { theme.tokens.danger }))
            .bg(rgb(theme.tokens.ground))
            .child(
                field::render(&self.hex, "#rrggbb", focused, &theme, field::on_press(cx, |_, _, _, _, _| {})).flex_1(),
            );
        let card = div()
            .key_context("ColorPicker")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::key_down))
            .w(theme.ui(300.))
            .max_w_full()
            .flex()
            .flex_col()
            .gap(theme.ui(12.))
            .p(theme.ui(18.))
            .bg(rgb(theme.tokens.surface))
            .border_1()
            .border_color(rgb(theme.tokens.line))
            .rounded(px(RADIUS_LARGE))
            .shadow_lg()
            .text_sm()
            .text_color(rgb(theme.tokens.text))
            .child(div().text_size(theme.ui(16.)).font_weight(FontWeight::SEMIBOLD).child(title))
            .child(square)
            .child(bar)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(theme.ui(8.))
                    .child(div().flex_none().size(theme.ui(32.)).rounded(px(RADIUS_SMALL)).bg(rgb(self.rgb())))
                    .child(hex),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(theme.ui(8.))
                    .child(
                        button("color-cancel", "Cancel", false, &theme)
                            .on_click(cx.listener(|_, _: &ClickEvent, _, cx| cx.emit(ColorPickerEvent::Cancelled))),
                    )
                    .child(
                        button("color-apply", "Apply", true, &theme)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.apply(cx))),
                    ),
            );
        fade_in(card, "fade-color-picker", FadeKind::Card, cx.global::<SettingsStore>().settings().animate_ui)
    }
}
