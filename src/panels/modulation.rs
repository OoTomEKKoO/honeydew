//! The GM Synth's free modulation routes as the panels show them (Honeydew change): the ring arc on a routed knob and
//! the inspector that a knob's `Modulate…` menu item opens.
//!
//! The routes themselves live in the panel's settings (`honeydew_audio::gmsynth::MOD_BASE`, four slots of source,
//! target and amount) so they are saved in the module's `CSTM` blob and reach the engine like any other setting. This
//! module only reads and writes those settings; the engine applies them.

use eframe::egui::{self, Align2, Area, Frame, Id, Order, Pos2, Ui, vec2};
use honeydew_audio::gmsynth::{
    MOD_AMOUNT, MOD_BASE, MOD_SLOTS, MOD_SOURCE, MOD_SOURCES, MOD_TARGET, MOD_TARGET_NAMES,
    MOD_TARGETS,
};

use super::PanelState;
use crate::theme::{Accent, ThemeColors};
use crate::ui::UiCtx;
use crate::ui::widgets::Modulation;

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

/// The index into the engine's target list of param `param`, if a route may drive it.
pub(super) fn target_of(param: usize) -> Option<usize> {
    MOD_TARGETS.iter().position(|&p| p == param)
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

/// The ring arc of param `param`'s knob, if a route drives it.
pub(super) fn modulation_of(
    state: &PanelState,
    colors: &ThemeColors,
    param: usize,
) -> Option<Modulation> {
    let route = route_on(state, param)?;
    Some(Modulation {
        amount: route.amount,
        color: source_accent(route.source).color(colors),
    })
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

/// The inspector for `param`'s knob, shown at `at` while `state.inspect` names it: the source (Off, LFO, EG), the bipolar
/// amount and a close button. Returns the route settings the user changed this frame.
pub(super) fn inspector(ui: &mut Ui, cx: &UiCtx, state: &mut PanelState, id: Id) -> Vec<u32> {
    let Some((param, at)) = state.inspect else {
        return Vec::new();
    };
    let Some(target) = target_of(param) else {
        state.inspect = None;
        return Vec::new();
    };
    let slot = slot_for(state, param);
    let mut changed = Vec::new();
    let mut close = false;
    let m = cx.m;
    let size = vec2(m.u(7.0), m.u(1.2));
    let screen = ui.ctx().content_rect();
    let at = Pos2::new(
        at.x.clamp(screen.left() + size.x / 2.0, screen.right() - size.x / 2.0),
        at.y.clamp(screen.top() + m.u(2.0), screen.bottom() - m.u(4.0)),
    );
    Area::new(id.with("inspector"))
        .order(Order::Foreground)
        .fixed_pos(at)
        .pivot(Align2::CENTER_CENTER)
        .show(ui.ctx(), |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(size.x);
                ui.label(format!("Modulate {}", MOD_TARGET_NAMES[target]));
                let Some(slot) = slot else {
                    ui.label("All four routes are in use (MATRIX page).");
                    close |= ui.button("Close").clicked();
                    return;
                };
                let current = setting(state, slot, MOD_SOURCE, 0.0) as usize;
                let on_us = setting(state, slot, MOD_TARGET, 0.0) as usize == target;
                let current = if on_us { current } else { 0 };
                ui.horizontal(|ui| {
                    for (i, name) in MOD_SOURCES.iter().enumerate() {
                        let button = egui::Button::new(*name)
                            .selected(current == i)
                            .min_size(vec2(size.x / 3.0 - 6.0, size.y));
                        if ui.add(button).clicked() && current != i {
                            changed.extend(set_route(state, slot, param, i));
                        }
                    }
                });
                if current > 0 {
                    let mut amount = 2.0 * setting(state, slot, MOD_AMOUNT, 0.5) - 1.0;
                    ui.spacing_mut().slider_width = size.x - m.u(2.0);
                    let slider = egui::Slider::new(&mut amount, -1.0..=1.0).text("Amount");
                    if ui.add_sized(vec2(size.x, size.y), slider).changed() {
                        changed.extend(set_amount(state, slot, amount));
                    }
                }
                close |= ui
                    .add(egui::Button::new("Done").min_size(vec2(size.x, size.y)))
                    .clicked();
            });
        });
    // A tap outside closes it too.
    let outside = ui.ctx().input(|i| i.pointer.any_pressed())
        && ui
            .ctx()
            .pointer_interact_pos()
            .is_some_and(|p| !ui.ctx().is_pointer_over_area() && p.is_finite());
    if close || outside {
        state.inspect = None;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

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
