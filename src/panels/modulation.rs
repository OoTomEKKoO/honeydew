//! The GM Synth's free modulation routes as the panels show them (Honeydew change): the ring arc on a routed knob and
//! the inspector that a knob's `Modulate…` menu item opens.
//!
//! The routes themselves live in the panel's settings (`honeydew_audio::gmsynth::MOD_BASE`, four slots of source,
//! target and amount) so they are saved in the module's `CSTM` blob and reach the engine like any other setting. This
//! module only reads and writes those settings; the engine applies them.

use eframe::egui::{
    Align, Align2, Area, Id, Key, Margin, Order, Rect, Sense, TextEdit, Ui, pos2, vec2,
};
use honeydew_audio::gmsynth::{
    MOD_AMOUNT, MOD_BASE, MOD_SLOTS, MOD_SOURCE, MOD_SOURCES, MOD_TARGET, MOD_TARGET_NAMES,
    MOD_TARGETS,
};

use super::PanelState;
use crate::metrics::FontSize;
use crate::theme::{Accent, ThemeColors};
use crate::ui::widgets::{self, Control, Modulation, ParamMenu};
use crate::ui::{UiCtx, close_pressed, dialog};

/// The module type this applies to: the GM Synth.
pub(super) const GMSYNTH_TYPE: u16 = 2;

/// The amount a new route starts at (0.5 is none): a quarter of the knob's travel upwards, so it shows at once.
const NEW_AMOUNT: f32 = 0.75;

/// A route as the panel reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RouteView {
    pub slot: usize,
    /// 1 the LFO, 2 the EG.
    pub source: usize,
    /// −1…1.
    pub amount: f32,
}

fn key(slot: usize, field: u32) -> u32 {
    MOD_BASE + 10 * slot as u32 + field
}

fn setting(state: &PanelState, slot: usize, field: u32, default: f32) -> f32 {
    let v = state.settings.get(&key(slot, field)).copied();
    v.filter(|v| v.is_finite()).unwrap_or(default)
}

/// The target menu item of param `param` (1 for the first target; 0 is `Off`), if a route may drive it.
pub(super) fn target_of(param: usize) -> Option<usize> {
    MOD_TARGETS.iter().position(|&p| p == param).map(|i| i + 1)
}

/// The first live route (a source, a target and an amount away from 0) that drives `param`.
pub(super) fn route_on(state: &PanelState, param: usize) -> Option<RouteView> {
    let target = target_of(param)?;
    (0..MOD_SLOTS).find_map(|slot| {
        let source = setting(state, slot, MOD_SOURCE, 0.0) as usize;
        let t = setting(state, slot, MOD_TARGET, 0.0) as usize;
        let amount = 2.0 * setting(state, slot, MOD_AMOUNT, 0.5).clamp(0.0, 1.0) - 1.0;
        (source > 0 && t == target && amount != 0.0).then_some(RouteView {
            slot,
            source,
            amount,
        })
    })
}

/// The accent a source is drawn in: the LFO's or the EG's page colour.
fn source_accent(source: usize) -> Accent {
    if source == 2 {
        Accent::Envelope
    } else {
        Accent::Lfo
    }
}

/// The ring arc of param `param`'s knob, if a route drives it: a MATRIX route first, else the LFO page's own route (its DEST and
/// AMOUNT), else the EG page's. `now` is left for the caller to fill from the engine's live values.
pub(super) fn modulation_of(
    state: &PanelState,
    colors: &ThemeColors,
    params: &[f32],
    param: usize,
) -> Option<Modulation> {
    let ring = |amount: f32, source: usize| Modulation {
        amount,
        color: source_accent(source).color(colors),
        now: None,
    };
    if let Some(route) = route_on(state, param) {
        return Some(ring(route.amount, route.source));
    }
    if let Some(amount) = page_range(params, param, 1) {
        return Some(ring(amount, 1));
    }
    page_range(params, param, 2).map(|amount| ring(amount, 2))
}

/// The engine's value of knob `param` now, from its live values `slab` ([`honeydew_audio::gmsynth::LIVE_TARGETS`]).
pub(super) fn live_value(slab: Option<&[f32]>, param: usize) -> Option<f32> {
    let i = MOD_TARGETS.iter().position(|&p| p == param)?;
    slab?
        .get(honeydew_audio::gmsynth::LIVE_TARGETS + i)
        .copied()
        .filter(|v| v.is_finite())
}

/// How far the LFO page's (`source` 1) or the EG page's (2) own route moves knob `param`, as a fraction of its travel (−1…1):
/// the LFO's DEST item (FLT, MOD, FREQ, LVL, PAN) with its AMOUNT, the EG's DEST (Filter, Osc. Mod., Pitch) with its
/// AMOUNT, by the laws the engine plays them with (a cube for the filter, a signed square for the mods, dB for the level).
/// `None` when it does not drive the knob or its AMOUNT is at 0.
pub(super) fn page_range(params: &[f32], param: usize, source: usize) -> Option<f32> {
    let get = |p: usize| params.get(p).copied().filter(|v| v.is_finite());
    let (amount, dest) = if source == 1 {
        (get(34)?, get(35)?)
    } else {
        (get(40)?, get(41)?)
    };
    let a = 2.0 * amount.clamp(0.0, 1.0) - 1.0;
    let squared = a * a.abs();
    let (driven, range) = if source == 1 {
        match ((4.0 * dest.clamp(0.0, 1.0)) as usize).min(4) {
            0 => (21, a.powi(3)),
            1 => (15, squared),
            3 => (1, 12.0 * a / 66.0),
            4 => (2, squared),
            _ => return None,
        }
    } else {
        match ((2.0 * dest.clamp(0.0, 1.0)) as usize).min(2) {
            0 => (21, a.powi(3)),
            1 => (15, squared),
            _ => return None,
        }
    };
    (param == driven && range.abs() >= 0.005).then_some(range.clamp(-1.0, 1.0))
}

/// The slot the inspector edits for `param`: the one that drives it, else the first free one.
fn slot_for(state: &PanelState, param: usize) -> Option<usize> {
    let target = target_of(param)?;
    let driving = |slot: usize| {
        setting(state, slot, MOD_SOURCE, 0.0) > 0.0
            && setting(state, slot, MOD_TARGET, 0.0) as usize == target
    };
    (0..MOD_SLOTS)
        .find(|&s| driving(s))
        .or_else(|| (0..MOD_SLOTS).find(|&s| setting(state, s, MOD_SOURCE, 0.0) == 0.0))
}

/// Writes `value` into a route's setting; returns its key when that changed it.
fn put(state: &mut PanelState, slot: usize, field: u32, value: f32) -> Option<u32> {
    let k = key(slot, field);
    let old = state.settings.insert(k, value);
    (old != Some(value)).then_some(k)
}

/// Sets route `slot` to `source` on `param` (source 0 removes it), keeping or starting an amount. Returns the settings
/// that changed.
pub(super) fn set_route(
    state: &mut PanelState,
    slot: usize,
    param: usize,
    source: usize,
) -> Vec<u32> {
    let Some(target) = target_of(param) else {
        return Vec::new();
    };
    let amount = if source == 0 {
        0.5
    } else if setting(state, slot, MOD_AMOUNT, 0.5) == 0.5 {
        NEW_AMOUNT
    } else {
        setting(state, slot, MOD_AMOUNT, 0.5)
    };
    [
        put(state, slot, MOD_SOURCE, source as f32),
        put(state, slot, MOD_TARGET, target as f32),
        put(state, slot, MOD_AMOUNT, amount),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Sets route `slot`'s bipolar amount (−1…1). Returns the settings that changed.
pub(super) fn set_amount(state: &mut PanelState, slot: usize, amount: f32) -> Vec<u32> {
    let v = (0.5 * (amount.clamp(-1.0, 1.0) + 1.0)).clamp(0.0, 1.0);
    put(state, slot, MOD_AMOUNT, v).into_iter().collect()
}

/// The inspector's box in U: wide enough for three source buttons, nine high.
const BOX: [f32; 2] = [17.0, 11.4];
const TITLE: FontSize = FontSize::Pt18;

/// What the LFO page's own route (its DEST and AMOUNT, the original's one fixed destination, which is not a MATRIX route)
/// does to param `param`: its amount as a percent of the range (−100…100), or `None` when it does not drive that knob or
/// its AMOUNT is at 0. DEST items: 0 the filter (CUTOFF), 1 the oscillator mod (MOD. AMNT), 2 the pitch, 3 the level, 4 the pan.
pub(super) fn page_lfo_on(params: &[f32], param: usize) -> Option<f32> {
    let get = |p: usize| params.get(p).copied().filter(|v| v.is_finite());
    let (amount, dest) = (get(34)?, get(35)?);
    let driven = match ((4.0 * dest.clamp(0.0, 1.0)) as usize).min(4) {
        0 => 21,
        1 => 15,
        3 => 1,
        4 => 2,
        _ => return None,
    };
    let percent = 100.0 * (2.0 * amount.clamp(0.0, 1.0) - 1.0);
    (param == driven && percent.abs() >= 1.0).then_some(percent)
}

/// The amount as the typed text reads it: a number of percent, `+`, `%` and spaces allowed, −100…100; `None` for text
/// that is no number.
pub(super) fn parse_percent(text: &str) -> Option<f32> {
    let t = text.trim().trim_end_matches('%').trim().replace(',', ".");
    let v: f32 = t.trim_start_matches('+').parse().ok()?;
    v.is_finite().then(|| (v / 100.0).clamp(-1.0, 1.0))
}

/// The amount's text, `+37 %` or `-100 %`.
pub(super) fn percent_text(amount: f32) -> String {
    format!("{:+.0} %", (100.0 * amount).round())
}

/// The inspector for `param`'s knob, in the app's dialog look (as the fine tune dialog's): the title, the source
/// (Off, LFO, EG) as three buttons, the bipolar amount as a slider, and under it the amount as text that a tap turns
/// into a field for typing a percent (the keyboard opens). Returns the route settings the user changed this frame.
pub(super) fn inspector(
    ui: &mut Ui,
    cx: &UiCtx,
    state: &mut PanelState,
    params: &[f32],
    id: Id,
) -> Vec<u32> {
    let Some((param, _)) = state.inspect else {
        return Vec::new();
    };
    let Some(target) = target_of(param) else {
        state.inspect = None;
        return Vec::new();
    };
    let ctx = ui.ctx().clone();
    let (m, c) = (cx.m, &cx.theme.colors);
    let ink = c.text.to_color32();
    let screen = ctx.content_rect();
    let size = vec2(m.u(BOX[0]).min(screen.width() - m.u(1.0)), m.u(BOX[1]));
    let min = screen.center() - size / 2.0;
    let rect = Rect::from_min_size(pos2(m.round_px(min.x), m.round_px(min.y)), size);
    let id = id.with("inspector");
    let backdrop = dialog::dialog_box(&ctx, id, (c, m), screen, rect);
    let slot = slot_for(state, param);
    let mut changed = Vec::new();
    let mut close = backdrop.clicked() || backdrop.secondary_clicked() || close_pressed(&ctx);
    let pad = m.u(1.0);
    let row = |top: f32, h: f32| {
        Rect::from_x_y_ranges(
            rect.left() + pad..=rect.right() - pad,
            rect.top() + m.u(top)..=rect.top() + m.u(top + h),
        )
    };
    let field_id = id.with("field");
    let mut typing = state.inspect_text.take();
    let mut apply = None;
    Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(rect.min)
        .constrain(false)
        .fade_in(false)
        .show(&ctx, |ui| {
            ui.allocate_exact_size(rect.size(), Sense::hover());
            let title = format!("Modulate {}", MOD_TARGET_NAMES[target]);
            widgets::shrink(
                ui.painter(),
                m,
                row(0.5, 1.5),
                Align2::CENTER_CENTER,
                &title,
                TITLE.points() as u8,
                ink,
            );
            // The LFO page's own route is not a MATRIX route and keeps working with every route off: say so.
            if let Some(percent) = page_lfo_on(params, param) {
                let note = format!(
                    "The LFO page also drives this knob ({percent:+.0} %): set its AMOUNT to the middle to stop it."
                );
                widgets::shrink(
                    ui.painter(),
                    m,
                    row(7.9, 0.9),
                    Align2::CENTER_CENTER,
                    &note,
                    8,
                    ink.gamma_multiply(0.75),
                );
            }
            // `Done` closes it (a tap outside or Escape do too).
            let done = row(9.2, 1.3);
            let done = Rect::from_center_size(done.center(), vec2(m.u(5.0), done.height()));
            let b = dialog::button(id.with("done"), done, "Done", c, m);
            if widgets::button(ui, cx, &b, true).clicked() {
                close = true;
            }
            let Some(slot) = slot else {
                let text = "All four routes are in use (MATRIX page).";
                ui.painter().text(
                    row(3.0, 2.0).center(),
                    Align2::CENTER_CENTER,
                    text,
                    m.font_id(FontSize::Pt12),
                    ink,
                );
                return;
            };
            let on_us = setting(state, slot, MOD_TARGET, 0.0) as usize == target;
            let current = if on_us {
                setting(state, slot, MOD_SOURCE, 0.0) as usize
            } else {
                0
            };
            // The three sources, solid buttons side by side.
            let sources = row(2.4, 1.4);
            let gap = m.u(0.3);
            let w = (sources.width() - 2.0 * gap) / 3.0;
            for (i, name) in MOD_SOURCES.iter().enumerate() {
                let x = sources.left() + i as f32 * (w + gap);
                let at = Rect::from_x_y_ranges(x..=x + w, sources.y_range());
                let b = dialog::button(id.with(("source", i)), at, name, c, m);
                if widgets::button(ui, cx, &b, current == i).clicked() && current != i {
                    changed.extend(set_route(state, slot, param, i));
                }
            }
            if current == 0 {
                let hint = "Choose LFO or EG to modulate this knob.";
                ui.painter().text(
                    row(4.6, 2.0).center(),
                    Align2::CENTER_CENTER,
                    hint,
                    m.font_id(FontSize::Pt10),
                    ink.gamma_multiply(0.7),
                );
                return;
            }
            // The amount: a slider centred on 0, and its text.
            let amount = 2.0 * setting(state, slot, MOD_AMOUNT, 0.5) - 1.0;
            let mut v = 0.5 * (amount + 1.0);
            let accent = source_accent(current).color(c);
            let control = Control {
                menu: ParamMenu::Off,
                ..Control::new(id.with("amount"), row(4.4, 1.6), "", accent, 0.5)
            };
            let text = |v: f32| percent_text(2.0 * v - 1.0);
            if widgets::fine_tune_slider(ui, cx, &control, &mut v, text).changed() {
                changed.extend(set_amount(state, slot, 2.0 * v - 1.0));
            }
            let value = row(6.2, 1.6);
            if let Some(buffer) = typing.as_mut() {
                let field = value.shrink2(vec2(m.u(4.0), 0.0));
                ui.painter()
                    .rect_filled(field, 0.0, c.background.to_color32());
                let edit = TextEdit::singleline(buffer)
                    .id(field_id)
                    .frame(false)
                    .margin(Margin::symmetric(m.u(0.25) as i8, 0))
                    .font(m.font_id(FontSize::Pt12))
                    .text_color(ink)
                    .desired_width(f32::INFINITY)
                    .min_size(field.size())
                    .horizontal_align(Align::Center)
                    .vertical_align(Align::Center);
                let mut response = ui.put(field, edit);
                crate::android_text::edit(&mut response, buffer, usize::MAX);
                if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                    apply = Some(buffer.clone());
                } else if !response.has_focus() && !response.lost_focus() {
                    // The field takes the keyboard as soon as it shows.
                    response.request_focus();
                }
            } else {
                let tap = ui.interact(value, id.with("value"), Sense::click());
                ui.painter().text(
                    value.center(),
                    Align2::CENTER_CENTER,
                    percent_text(amount),
                    m.font_id(TITLE),
                    ink,
                );
                if tap.clicked() {
                    typing = Some(format!("{:.0}", 100.0 * amount));
                }
            }
        });
    // Enter takes the typed percent (text that is no number is dropped), then the field closes.
    if let (Some(text), Some(slot)) = (apply, slot) {
        if let Some(amount) = parse_percent(&text) {
            changed.extend(set_amount(state, slot, amount));
        }
        typing = None;
        ctx.memory_mut(|mem| mem.surrender_focus(field_id));
    }
    if close {
        typing = None;
        state.inspect = None;
        ctx.memory_mut(|mem| mem.surrender_focus(field_id));
    }
    state.inspect_text = typing;
    ctx.request_repaint();
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lfo_pages_own_route_is_named_for_the_knob_it_drives() {
        // AMOUNT (34) at the middle is none; DEST (35) FLT is the cutoff (21), MOD the mod amount (15), LVL the level
        // (1) and PAN the pan (2); FREQ is a pitch and has no knob.
        let params = |amount: f32, dest: f32| {
            let mut p = vec![0.5; 77];
            (p[34], p[35]) = (amount, dest);
            p
        };
        assert_eq!(page_lfo_on(&params(0.5, 0.0), 21), None);
        assert_eq!(page_lfo_on(&params(1.0, 0.0), 21), Some(100.0));
        assert_eq!(page_lfo_on(&params(0.25, 0.0), 21), Some(-50.0));
        assert_eq!(page_lfo_on(&params(1.0, 0.0), 22), None, "another knob");
        for (dest, knob) in [(0.25, 15), (0.75, 1), (1.0, 2)] {
            assert_eq!(
                page_lfo_on(&params(1.0, dest), knob),
                Some(100.0),
                "dest {dest}"
            );
        }
        assert_eq!(
            page_lfo_on(&params(1.0, 0.5), 21),
            None,
            "FREQ drives a pitch"
        );
        assert_eq!(page_lfo_on(&[], 21), None);
    }

    #[test]
    fn the_pages_own_routes_give_a_ring_by_the_engines_laws() {
        let mut p = vec![0.5; 77];
        // LFO page: AMOUNT +100 % on FLT is the cutoff with a cubed depth of 1; at +50 % it is 1/8 of the travel.
        (p[34], p[35]) = (1.0, 0.0);
        assert_eq!(page_range(&p, 21, 1), Some(1.0));
        p[34] = 0.75;
        assert!((page_range(&p, 21, 1).unwrap() - 0.125).abs() < 1e-6);
        // MOD is a signed square, LVL is twelve dB of the 66 dB level range, PAN a signed square.
        (p[34], p[35]) = (1.0, 0.25);
        assert_eq!(page_range(&p, 15, 1), Some(1.0));
        p[35] = 0.75;
        assert!((page_range(&p, 1, 1).unwrap() - 12.0 / 66.0).abs() < 1e-6);
        p[35] = 1.0;
        assert_eq!(page_range(&p, 2, 1), Some(1.0));
        // AMOUNT in the middle is none, and so is FREQ (a pitch).
        p[34] = 0.5;
        assert_eq!(page_range(&p, 2, 1), None);
        (p[34], p[35]) = (1.0, 0.5);
        assert_eq!(page_range(&p, 21, 1), None);
        // The EG page: Filter, Osc. Mod. and Pitch; AMOUNT 40, DEST 41.
        let mut q = vec![0.5; 77];
        (q[40], q[41]) = (1.0, 0.0);
        assert_eq!(page_range(&q, 21, 2), Some(1.0));
        q[41] = 0.5;
        assert_eq!(page_range(&q, 15, 2), Some(1.0));
        q[41] = 1.0;
        assert_eq!(page_range(&q, 21, 2), None, "pitch has no knob");
    }

    #[test]
    fn a_matrix_route_wins_over_the_pages_and_live_values_come_from_the_slab() {
        let colors = crate::theme::theme_by_id(0).colors;
        let mut state = PanelState::default();
        let mut params = vec![0.5; 77];
        (params[34], params[35]) = (1.0, 0.0);
        let page = modulation_of(&state, &colors, &params, 21).unwrap();
        assert_eq!(page.color, source_accent(1).color(&colors));
        set_route(&mut state, 0, 21, 2);
        let matrix = modulation_of(&state, &colors, &params, 21).unwrap();
        assert_eq!(
            matrix.color,
            source_accent(2).color(&colors),
            "the matrix route's source"
        );
        assert!(modulation_of(&state, &colors, &params, 22).is_none());
        let mut slab = vec![0.0; honeydew_audio::gmsynth::LIVE_LEN];
        let i = MOD_TARGETS.iter().position(|&p| p == 21).unwrap();
        slab[honeydew_audio::gmsynth::LIVE_TARGETS + i] = 0.8;
        assert_eq!(live_value(Some(&slab), 21), Some(0.8));
        assert_eq!(live_value(Some(&slab[..10]), 21), None, "a short slab");
        assert_eq!(live_value(None, 21), None);
        assert_eq!(
            live_value(Some(&slab), 3),
            None,
            "a knob that cannot be routed"
        );
    }

    #[test]
    fn typed_percents_are_read_leniently_and_clamped() {
        for (text, want) in [
            ("37", Some(0.37)),
            (" -50 % ", Some(-0.5)),
            ("+12,5", Some(0.125)),
            ("250", Some(1.0)),
            ("-1000", Some(-1.0)),
            ("", None),
            ("abc", None),
            ("NaN", None),
        ] {
            assert_eq!(parse_percent(text), want, "{text:?}");
        }
        assert_eq!(percent_text(0.374), "+37 %");
        assert_eq!(percent_text(-1.0), "-100 %");
    }

    fn cutoff() -> usize {
        21
    }

    #[test]
    fn a_new_route_shows_on_its_knob_and_removing_it_clears_the_ring() {
        let mut state = PanelState::default();
        assert_eq!(route_on(&state, cutoff()), None);
        let slot = slot_for(&state, cutoff()).unwrap();
        assert_eq!(slot, 0, "the first free slot");
        let changed = set_route(&mut state, slot, cutoff(), 1);
        assert_eq!(changed.len(), 3, "source, target and the starting amount");
        let view = route_on(&state, cutoff()).unwrap();
        assert_eq!((view.slot, view.source), (0, 1));
        assert!((view.amount - 0.5).abs() < 1e-6, "{}", view.amount);
        // Another knob is not affected, a second knob takes the next slot.
        assert_eq!(route_on(&state, 22), None);
        assert_eq!(slot_for(&state, 22), Some(1));
        set_route(&mut state, 1, 22, 2);
        assert_eq!(route_on(&state, 22).unwrap().source, 2);
        // The inspector edits the route that already drives the knob.
        assert_eq!(slot_for(&state, cutoff()), Some(0));
        // Off removes it.
        set_route(&mut state, 0, cutoff(), 0);
        assert_eq!(route_on(&state, cutoff()), None);
    }

    #[test]
    fn the_amount_is_bipolar_around_the_centre() {
        let mut state = PanelState::default();
        set_route(&mut state, 0, cutoff(), 1);
        set_amount(&mut state, 0, -1.0);
        assert_eq!(route_on(&state, cutoff()).unwrap().amount, -1.0);
        set_amount(&mut state, 0, 0.0);
        assert_eq!(route_on(&state, cutoff()), None, "centre is no modulation");
        set_amount(&mut state, 0, 9.0);
        assert_eq!(route_on(&state, cutoff()).unwrap().amount, 1.0);
    }

    #[test]
    fn only_listed_params_can_be_routed_and_four_routes_fill_up() {
        assert_eq!(target_of(3), None, "a menu or envelope time is no target");
        assert!(set_route(&mut PanelState::default(), 0, 3, 1).is_empty());
        let mut state = PanelState::default();
        for (slot, param) in [21, 22, 1, 2].into_iter().enumerate() {
            set_route(&mut state, slot, param, 1);
        }
        assert_eq!(slot_for(&state, 11), None, "no free slot left");
        assert_eq!(
            slot_for(&state, 21),
            Some(0),
            "but a driven knob can be edited"
        );
    }
}
