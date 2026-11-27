//! The GMSynth panel (type 2), card 9.5 U: eight tabs in the order of the signal (LEVEL, OSC, MOD, FILTER, EG, LFO, VOICES,
//! FX), and on `FX` one page per effect slot.
//! Ids follow the instrument convention, tab·10000 plus the `#n` digits padded to four: FILTER #6.1 is 16100.

use eframe::egui::{
    Align2, Color32, Id, Pos2, Rect, Sense, Shape, Stroke, Ui, epaint::Mesh, pos2, vec2,
};
use honeydew_audio::gm_waveshapes::TABLE_LEN;
use honeydew_audio::gmsynth::EnvelopeKind;

use crate::metrics::FontSize;
use crate::panels::LOWER_TABS;
use crate::panels::build::*;
use crate::panels::{ModuleInstance, PanelState};
use crate::ui::{UiCtx, widgets};
// The waveshapes of the OSC `A` and `B` spinners, kept with the preset, not as params.
use crate::params::GMSYNTH_WAVESHAPES as WAVESHAPES;

const PB_RANGE: &[&str] = &["1 smt", "2 smt", "7 smt", "Oct"];
const MW_DEST: &[&str] = &["Filter", "LFO", "Osc. Mod."];
/// Honeydew's FM depth range, a `CSTM` setting; `x1` is the original depth.
const FM_RANGES: &[&str] = &["x1", "x4", "x8", "x16"];
/// Honeydew's extra filter types, a `CSTM` setting; `Off` keeps the `KIND` spinner's filter.
const FILTER_EXTRA: &[&str] = &["Off", "BP", "Notch", "Comb"];

const TABS: &str = "LEVEL · OSC · MOD · FILTER · EG · LFO · VOICES · FX";

/// A page of rows of controls, top to bottom.
fn page(rows: impl IntoIterator<Item = Part>) -> Part {
    vstack(rows)
}

/// A row of controls, left to right.
fn line(parts: impl IntoIterator<Item = Part>) -> Part {
    hstack(parts)
}

/// The height of the slim row of compact menus and switches under the knobs, in U.
const MENUS: f32 = 1.4;

/// A knob row's share of a page's height, against its picture's.
fn knobs(parts: impl IntoIterator<Item = Part>, share: f32) -> Part {
    line(parts).h(Size::Flex(share))
}

/// A picture of what the page's controls do, in a well of its own.
fn picture(id: u32, draw: crate::panels::Draw, share: f32) -> Part {
    line([display(id).draw(draw).margins(0.1, 0.1, 0.1, 0.1)]).h(Size::Flex(share))
}

pub fn panel() -> Option<Panel> {
    // Pages under two tab strips: with two pages at once (a tall portrait card) each has a strip of its own. A picture
    // on top of what it shows, the knobs under it (input below, as on a phone's keyboard), and the menus and switches in
    // a slim row at the foot.
    Some(Panel::new(pager(
        tabs(0, TABS).h(Size::U(1.25)),
        tabs(LOWER_TABS, TABS).h(Size::U(1.25)),
        vec![
            // LEVEL
            page([
                picture(41100, env_level, 2.2),
                knobs(
                    [
                        knob(41000, "LEVEL").p(1),
                        knob(42000, "PAN").p(2),
                        knob(43000, "ATK").p(3),
                        knob(44000, "DEC").p(4),
                        knob(45000, "SUS").p(5),
                        knob(46000, "REL").p(6),
                    ],
                    1.5,
                ),
            ]),
            // OSC: each oscillator is its own menu, a live picture of the wave whose tap opens the shape list.
            page([
                line([
                    spinner(21100, "A")
                        .items(WAVESHAPES)
                        .items_per_column(10)
                        .draw(wave_a)
                        .margins(0.1, 0.1, 0.1, 0.1),
                    spinner(21200, "B")
                        .items(WAVESHAPES)
                        .items_per_column(10)
                        .draw(wave_b)
                        .margins(0.1, 0.1, 0.1, 0.1),
                ])
                .h(Size::Flex(1.9)),
                knobs(
                    [
                        knob(22000, "PITCH B").p(11),
                        knob(23000, "FINE B").p(12),
                        knob(24000, "PHASE B").p(14),
                        knob(25000, "BALANCE").p(13),
                        knob(26000, "NOISE").p(20),
                    ],
                    1.5,
                ),
            ]),
            // MOD
            page([
                picture(31100, env_mod, 2.0),
                knobs(
                    [
                        knob(31000, "MOD. AMNT").p(15),
                        knob(33000, "ATTACK").p(17),
                        knob(34000, "DECAY").p(18),
                        knob(35000, "EG AMOUNT").p(19),
                        knob(37000, "MW AMOUNT").default(0.5),
                    ],
                    1.5,
                ),
                line([
                    spinner(32000, "KIND").p(16).compact(),
                    // Honeydew's own: how far FM modulates (`x1` is the original depth).
                    spinner(32100, "FM RANGE").items(FM_RANGES).compact(),
                    // A new GMSynth's `CSTM` (`cstm::GMSYNTH`, which owns the encoding): `2 smt`, `Osc. Mod.`, mid.
                    spinner(36100, "PB RANGE")
                        .items(PB_RANGE)
                        .default(1.0)
                        .compact(),
                    spinner(36200, "MW DEST")
                        .items(MW_DEST)
                        .default(2.0)
                        .compact(),
                ])
                .h(Size::U(MENUS)),
            ]),
            // FILTER
            page([
                picture(12100, filter_display, 2.4),
                knobs(
                    [
                        knob(11000, "CUTOFF").p(21),
                        knob(12000, "RES").p(22),
                        knob(13000, "ATTACK").p(25),
                        knob(14000, "DECAY").p(26),
                        knob(15000, "AMOUNT").p(27),
                    ],
                    1.7,
                ),
                line([
                    text_toggle(16100, "KBD").p(24).w(Size::Flex(0.45)),
                    spinner(16200, "KIND").p(23).compact(),
                    // Honeydew's own extra filter types.
                    spinner(16300, "EXTRA").items(FILTER_EXTRA).compact(),
                ])
                .h(Size::U(MENUS)),
            ]),
            // EG
            page([
                picture(51100, env_eg, 2.0),
                knobs(
                    [
                        knob(51000, "AMOUNT").p(40),
                        knob(52000, "ATTACK").p(36),
                        knob(53000, "DECAY").p(37),
                        knob(54000, "SUSTAIN").p(38),
                        knob(55000, "RELEASE").p(39),
                    ],
                    1.5,
                ),
                line([
                    text_toggle(56100, "VEL").p(74).w(Size::Flex(0.45)),
                    spinner(56200, "DEST").p(41).compact(),
                ])
                .h(Size::U(MENUS)),
            ]),
            // LFO
            page([
                picture(61100, lfo_display, 2.2),
                knobs(
                    [
                        knob(61000, "AMOUNT").p(34),
                        spinner(62000, "SHAPE").p(33).compact(),
                        spinner(63000, "DEST").p(35).compact(),
                        spinner(64000, "RATE").p(32).items_per_column(5).compact(),
                    ],
                    1.3,
                ),
            ]),
            // VOICES
            page([
                knobs(
                    [
                        knob(71000, "TRNSP").p(28),
                        knob(72000, "SLIDE").p(29),
                        knob(73000, "CHOIR DTN").p(44),
                        knob(74000, "CHOIR STR").p(45),
                    ],
                    1.0,
                ),
                line([
                    text_toggle(75100, "MONO").p(30).w(Size::Flex(0.6)),
                    text_toggle(75200, "AUTO").p(31).w(Size::Flex(0.6)),
                    spinner(76100, "VOICES").p(42).compact(),
                    spinner(76200, "SCALE").p(43).compact(),
                ])
                .h(Size::U(MENUS)),
            ]),
            fx(),
        ],
    )))
}

/// Partials the wave displays draw: enough for a saw's or a square's corners, few enough for no ringing.
const DISPLAY_HARMONICS: usize = 96;
/// Cycles of an oscillator a wave display shows: one whole cycle, so that no cycle is cut off by the display's edge.
const DISPLAY_CYCLES: f32 = 1.0;
/// The share of its half height the wave's peak reaches: the curve and its stroke stay inside the well.
const DISPLAY_FILL: f32 = 0.72;
/// A curve point per device pixel, at most this many.
const DISPLAY_MAX_POINTS: usize = 900;

/// Osc A's wave display: its shape as modulated by B (ring, FM or sync, by the MOD page), scaled to its BALANCE share.
fn wave_a(
    ui: &mut eframe::egui::Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    state: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    wave_display(ui, cx, rect, module, state, ink, false);
    Vec::new()
}

/// Osc B's wave display: its shape at its PHASE B, with its BALANCE share (inverted below the middle, as it sounds).
fn wave_b(
    ui: &mut eframe::egui::Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    state: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    wave_display(ui, cx, rect, module, state, ink, true);
    Vec::new()
}

/// One cycle of a shape, read at phase `x` (cycles, any sign) with linear interpolation.
fn read_cycle(cycle: &[f32], x: f32) -> f32 {
    let at = x.rem_euclid(1.0) * TABLE_LEN as f32;
    let (i, f) = ((at as usize) % TABLE_LEN, at.fract());
    cycle[i] + (cycle[(i + 1) % TABLE_LEN] - cycle[i]) * f
}

/// Osc A's curve at `steps + 1` points over [`DISPLAY_CYCLES`] cycles of A, as the engine plays it: plain, or ring
/// (A × (1 + 3·m·B)), FM (A read at φ_A + (B + 1)·m·scale cycles) or sync (A restarts when B wraps), by the MOD page's
/// KIND, MOD. AMNT and FM RANGE, with B at its PITCH B, FINE B and PHASE B. The modulation by envelopes, the LFO and the
/// wheel is not drawn.
fn curve_a(
    a: &[f32],
    b: &[f32],
    module: &ModuleInstance,
    state: &PanelState,
    steps: usize,
) -> Vec<f32> {
    let param = |p: usize| module.params.get(p).copied().unwrap_or(0.5).clamp(0.0, 1.0);
    let ratio = f64::from(honeydew_audio::gmsynth::osc_b_ratio(param(11), param(12)));
    let (amount, offset) = (param(15), f64::from(param(14)));
    // KIND: int(3v) of Ring, FM, Sync, Filter.
    let kind = ((3.0 * param(16)) as u8).min(3);
    let scale = state
        .settings
        .get(&32100)
        .and_then(|&i| honeydew_audio::gmsynth::FM_SCALES.get(i as usize))
        .copied()
        .unwrap_or(1.0);
    let mut restart = 0.0_f64;
    let mut wraps = ((offset).floor()) as i64;
    (0..=steps)
        .map(|i| {
            let t = f64::from(DISPLAY_CYCLES) * i as f64 / steps as f64;
            let phase_b = t * ratio + offset;
            let b = read_cycle(b, phase_b.rem_euclid(1.0) as f32);
            match kind {
                0 if amount > 0.001 => read_cycle(a, t as f32) * (1.0 + 3.0 * amount * b),
                1 if amount > 0.001 => {
                    let shift = f64::from((b + 1.0) * amount * scale).rem_euclid(1.0);
                    read_cycle(a, (t + shift).rem_euclid(1.0) as f32)
                }
                2 => {
                    // B's phase wrapped since the last point: A starts over from zero.
                    if phase_b.floor() as i64 != wraps {
                        wraps = phase_b.floor() as i64;
                        restart = t;
                    }
                    read_cycle(a, (t - restart).rem_euclid(1.0) as f32)
                }
                _ => read_cycle(a, t as f32),
            }
        })
        .collect()
}

/// The shared drawing of the wave displays: a well, a faint zero line, the curve with a soft fill to the zero line.
/// A's curve is the oscillator as the MOD page modulates it ([`curve_a`]); B's is its own shape at its PHASE B. Both
/// shapes are the oscillators' menu settings (`#21100` / `#21200`); each curve is normalised to its own peak, so
/// it fills the well alike, and fades with the oscillator's BALANCE share, to a quarter at silence. A shape past the
/// menu (none yet) leaves the zero line only. One curve point per device pixel, so it stays smooth at any size.
fn wave_display(
    ui: &mut eframe::egui::Ui,
    cx: &UiCtx,
    rect: Rect,
    module: &ModuleInstance,
    state: &PanelState,
    ink: Color32,
    b: bool,
) {
    widgets::display_area(ui, cx, rect);
    let t = cx.m.t();
    // Room at the sides too, so that the curve never runs into the display's edge.
    let inner = rect.shrink2(vec2(7.0 * t, 3.0 * t));
    if inner.width() < 4.0 || inner.height() < 4.0 {
        return;
    }
    let painter = ui.painter();
    let mid = inner.center().y;
    let text = cx.theme.colors.text.to_color32();
    painter.line_segment(
        [pos2(inner.left(), mid), pos2(inner.right(), mid)],
        Stroke::new(t * 0.5, text.gamma_multiply(0.15)),
    );
    let param = |p: usize| module.params.get(p).copied().unwrap_or(0.5).clamp(0.0, 1.0);
    let shape = |key: u32| {
        let slot = *state.settings.get(&key)?;
        honeydew_audio::gmsynth::preview_cycle(slot as usize, DISPLAY_HARMONICS)
    };
    let slot = state
        .settings
        .get(&if b { 21200 } else { 21100 })
        .map(|&v| v as usize);
    let (Some(own), Some(other)) = (
        shape(if b { 21200 } else { 21100 }),
        shape(if b { 21100 } else { 21200 }),
    ) else {
        return;
    };
    // BALANCE: A takes 1 − |2v − 1| of it, B 2v − 1 (negative: B upside down).
    let balance = 2.0 * param(13) - 1.0;
    let (share, sign) = if b {
        (balance.abs(), balance.signum())
    } else {
        (1.0 - balance.abs(), 1.0)
    };
    let ink = ink.gamma_multiply(0.25 + 0.75 * share);
    let steps = ((inner.width() * cx.m.ppp()) as usize).clamp(48, DISPLAY_MAX_POINTS);
    let values: Vec<f32> = if b {
        let shift = f64::from(param(14));
        (0..=steps)
            .map(|i| {
                let x = f64::from(DISPLAY_CYCLES) * i as f64 / steps as f64;
                read_cycle(&own, (x + shift).rem_euclid(1.0) as f32)
            })
            .collect()
    } else {
        curve_a(&own, &other, module, state, steps)
    };
    let peak = values.iter().fold(1e-6_f32, |m, v| m.max(v.abs()));
    let amp = inner.height() / 2.0 * DISPLAY_FILL * sign / peak;
    let points: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                inner.left() + inner.width() * i as f32 / steps as f32,
                mid - v * amp,
            )
        })
        .collect();
    // The soft fill between the curve and the zero line, then the curve.
    let mut mesh = Mesh::default();
    let fill = ink.gamma_multiply(0.2);
    for pair in points.windows(2) {
        let [p, q] = [pair[0], pair[1]];
        let base = mesh.vertices.len() as u32;
        for corner in [pos2(p.x, mid), p, q, pos2(q.x, mid)] {
            mesh.colored_vertex(corner, fill);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(Shape::mesh(mesh));
    painter.add(Shape::line(points, Stroke::new(t * 1.25, ink)));
    caption(ui, cx, inner, if b { "B" } else { "A" }, slot, ink);
}

/// The display's caption in its top left corner, the oscillator and its shape's name, and a small chevron at the top
/// right that says a tap opens the list.
fn caption(ui: &Ui, cx: &UiCtx, inner: Rect, osc: &str, slot: Option<usize>, ink: Color32) {
    let painter = ui.painter();
    let m = cx.m;
    let name = slot.and_then(|i| WAVESHAPES.get(i)).copied().unwrap_or("");
    let text = cx.theme.colors.text.to_color32();
    painter.text(
        inner.left_top() + vec2(m.t() * 2.0, m.t() * 1.0),
        Align2::LEFT_TOP,
        format!("{osc}  {name}"),
        m.font_id(FontSize::Pt10),
        text.gamma_multiply(0.85),
    );
    let c = pos2(inner.right() - m.t() * 5.0, inner.top() + m.t() * 4.5);
    let w = m.t() * 2.5;
    painter.add(Shape::line(
        vec![
            c + vec2(-w, -w * 0.5),
            c + vec2(0.0, w * 0.5),
            c + vec2(w, -w * 0.5),
        ],
        Stroke::new(m.t(), ink),
    ));
}

/// A well and the room inside it, or `None` when it is too small to draw in.
fn well(ui: &Ui, cx: &UiCtx, rect: Rect) -> Option<Rect> {
    widgets::display_area(ui, cx, rect);
    let inner = rect.shrink(3.0 * cx.m.t());
    (inner.width() >= 4.0 && inner.height() >= 4.0).then_some(inner)
}

/// A curve with a soft fill between it and the horizontal line at `base`.
fn fill_curve(ui: &Ui, points: Vec<Pos2>, base: f32, ink: Color32, width: f32) {
    let painter = ui.painter();
    let mut mesh = Mesh::default();
    let fill = ink.gamma_multiply(0.2);
    for pair in points.windows(2) {
        let [p, q] = [pair[0], pair[1]];
        let first = mesh.vertices.len() as u32;
        for corner in [pos2(p.x, base), p, q, pos2(q.x, base)] {
            mesh.colored_vertex(corner, fill);
        }
        mesh.add_triangle(first, first + 1, first + 2);
        mesh.add_triangle(first, first + 2, first + 3);
    }
    painter.add(Shape::mesh(mesh));
    painter.add(Shape::line(points, Stroke::new(width, ink)));
}

/// A curve point per device pixel across `inner`, between 48 and 600.
fn curve_points(cx: &UiCtx, inner: Rect) -> usize {
    ((inner.width() * cx.m.ppp()) as usize).clamp(48, 600)
}

/// The frequency range the filter display covers, in Hz, and the gain range in dB.
const FILTER_HZ: (f32, f32) = (20.0, 20_000.0);
const FILTER_DB: (f32, f32) = (-30.0, 24.0);

/// The filter display: its gain over frequency on a log axis, as the engine's filter does it (KIND, EXTRA, CUTOFF and
/// RES), with the cutoff drawn as a line and the resonance as a ring where the two meet. Dragging moves them: the
/// pointer's frequency is the CUTOFF and its height the RES. The envelopes and the LFO that move the cutoff are not drawn.
fn filter_display(
    ui: &mut Ui,
    cx: &UiCtx,
    id: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    state: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let Some(inner) = well(ui, cx, rect) else {
        return Vec::new();
    };
    let t = cx.m.t();
    let log_span = (FILTER_HZ.1 / FILTER_HZ.0).ln();
    let x_of = |hz: f32| inner.left() + inner.width() * (hz / FILTER_HZ.0).ln() / log_span;
    let y_of = |db: f32| {
        let v = (db - FILTER_DB.0) / (FILTER_DB.1 - FILTER_DB.0);
        inner.bottom() - v.clamp(0.0, 1.0) * inner.height()
    };
    // A drag sets the cutoff (x) and the resonance (y): the pointer's place, not a move from the last one.
    let response = ui.interact(rect, id, Sense::drag());
    let mut changed = Vec::new();
    if response.dragged()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let x = ((pos.x - inner.left()) / inner.width()).clamp(0.0, 1.0);
        let hz = FILTER_HZ.0 * (FILTER_HZ.1 / FILTER_HZ.0).powf(x);
        let res = (1.0 - (pos.y - inner.top()) / inner.height()).clamp(0.0, 1.0);
        for (p, v) in [(21, honeydew_audio::gmsynth::cutoff_of_hz(hz)), (22, res)] {
            if let Some(slot) = module.params.get_mut(p)
                && (*slot - v).abs() > 1e-4
            {
                *slot = v;
                changed.push(p);
            }
        }
    }
    let param = |p: usize| module.params.get(p).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    let (cutoff, res) = (param(21), param(22));
    let kind = ((2.0 * param(23)) as u8).min(2);
    let extra = state.settings.get(&16300).map_or(0, |&v| (v as u8).min(3));
    let painter = ui.painter();
    let (text, grid) = (
        cx.theme.colors.text.to_color32(),
        cx.theme.colors.text.to_color32().gamma_multiply(0.12),
    );
    // The grid: 0 dB, and 100 Hz, 1 kHz and 10 kHz with their names.
    painter.line_segment(
        [
            pos2(inner.left(), y_of(0.0)),
            pos2(inner.right(), y_of(0.0)),
        ],
        Stroke::new(t * 0.5, grid),
    );
    for (hz, name) in [(100.0, "100"), (1_000.0, "1k"), (10_000.0, "10k")] {
        let x = x_of(hz);
        painter.line_segment(
            [pos2(x, inner.top()), pos2(x, inner.bottom())],
            Stroke::new(t * 0.5, grid),
        );
        painter.text(
            pos2(x + t, inner.bottom() - t),
            Align2::LEFT_BOTTOM,
            name,
            cx.m.font_id(FontSize::Pt8),
            text.gamma_multiply(0.45),
        );
    }
    let steps = curve_points(cx, inner);
    let freqs: Vec<f32> = (0..=steps)
        .map(|i| FILTER_HZ.0 * (FILTER_HZ.1 / FILTER_HZ.0).powf(i as f32 / steps as f32))
        .collect();
    let gains = honeydew_audio::gmsynth::filter_response(cutoff, res, kind, extra, &freqs);
    let points: Vec<Pos2> = freqs
        .iter()
        .zip(&gains)
        .map(|(&hz, &db)| pos2(x_of(hz), y_of(db)))
        .collect();
    // The cutoff line and, where it meets the curve, the resonance ring.
    let cx_hz = honeydew_audio::gmsynth::cutoff_hz(cutoff);
    let x = x_of(cx_hz).clamp(inner.left(), inner.right());
    painter.line_segment(
        [pos2(x, inner.top()), pos2(x, inner.bottom())],
        Stroke::new(t * 0.75, ink.gamma_multiply(0.5)),
    );
    fill_curve(ui, points, inner.bottom(), ink, t * 1.25);
    let handle = pos2(x, inner.bottom() - res * inner.height());
    ui.painter()
        .circle_stroke(handle, t * 3.0, Stroke::new(t, ink));
    changed
}

/// A time for a stage label: `0 ms`, `120 ms`, `1.2 s`, `12 s`.
fn seconds_text(s: f32) -> String {
    if s < 0.9995 {
        format!("{:.0} ms", s * 1000.0)
    } else if s < 9.95 {
        format!("{:.1} s", s)
    } else {
        format!("{:.0} s", s)
    }
}

/// The envelope display of `kind`: the engine's curves stage by stage, each over a fixed share of the width so that a
/// 2 ms attack and a 20 s decay both read, with a faint line between the stages and each stage's real time at its top.
fn env_display(
    ui: &Ui,
    cx: &UiCtx,
    rect: Rect,
    module: &ModuleInstance,
    ink: Color32,
    kind: EnvelopeKind,
) {
    let Some(inner) = well(ui, cx, rect) else {
        return;
    };
    let t = cx.m.t();
    let preview =
        honeydew_audio::gmsynth::envelope_preview(kind, &module.params, curve_points(cx, inner));
    // Room above for the times, and a little under the curve's foot and to its sides.
    let (left, right) = (inner.left() + 3.0 * t, inner.right() - 3.0 * t);
    let (top, bottom) = (inner.top() + inner.height() * 0.2, inner.bottom() - 2.0 * t);
    let last = (preview.values.len() - 1) as f32;
    let points: Vec<Pos2> = preview
        .values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                left + (right - left) * i as f32 / last,
                bottom - v * (bottom - top),
            )
        })
        .collect();
    let painter = ui.painter();
    let text = cx.theme.colors.text.to_color32();
    let x_of = |f: f32| left + (right - left) * f;
    let stages: &[(f32, f32, &str, f32)] = if preview.releases {
        &[
            (0.0, preview.ends[0], "A", preview.seconds[0]),
            (preview.ends[0], preview.ends[1], "D", preview.seconds[1]),
            (preview.ends[2], preview.ends[3], "R", preview.seconds[2]),
        ]
    } else {
        &[
            (0.0, preview.ends[0], "A", preview.seconds[0]),
            (preview.ends[0], preview.ends[1], "D", preview.seconds[1]),
        ]
    };
    // The line between stages, at the start of each stage after the first (and of the hold).
    let mut bounds = vec![preview.ends[0]];
    if preview.releases {
        bounds.extend([preview.ends[1], preview.ends[2]]);
    }
    for f in bounds {
        painter.line_segment(
            [pos2(x_of(f), inner.top()), pos2(x_of(f), inner.bottom())],
            Stroke::new(t * 0.5, text.gamma_multiply(0.12)),
        );
    }
    for &(from, _, name, seconds) in stages {
        painter.text(
            pos2(x_of(from) + 2.0 * t, inner.top() + t),
            Align2::LEFT_TOP,
            format!("{name} {}", seconds_text(seconds)),
            cx.m.font_id(FontSize::Pt8),
            text.gamma_multiply(0.55),
        );
    }
    fill_curve(ui, points, bottom, ink, t * 1.25);
}

/// The LFO display: two cycles of its shape, as tall as its AMOUNT (below the middle line when the amount is negative).
fn lfo_display(
    ui: &mut Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let Some(inner) = well(ui, cx, rect) else {
        return Vec::new();
    };
    let t = cx.m.t();
    let param = |p: usize| module.params.get(p).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    let amount = 2.0 * param(34) - 1.0;
    let values = honeydew_audio::gmsynth::lfo_preview(param(33), 2.0, curve_points(cx, inner));
    let mid = inner.center().y;
    let amp = inner.height() / 2.0 * 0.85 * amount;
    let last = (values.len() - 1) as f32;
    let points: Vec<Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            pos2(
                inner.left() + inner.width() * i as f32 / last,
                mid - v * amp,
            )
        })
        .collect();
    ui.painter().line_segment(
        [pos2(inner.left(), mid), pos2(inner.right(), mid)],
        Stroke::new(
            t * 0.5,
            cx.theme.colors.text.to_color32().gamma_multiply(0.15),
        ),
    );
    fill_curve(ui, points, mid, ink, t * 1.25);
    Vec::new()
}

/// The four envelope displays, one `Draw` each.
macro_rules! env_draw {
    ($name:ident, $kind:expr) => {
        fn $name(
            ui: &mut Ui,
            cx: &UiCtx,
            _: Id,
            rect: Rect,
            module: &mut ModuleInstance,
            _: &mut PanelState,
            ink: Color32,
        ) -> Vec<usize> {
            env_display(ui, cx, rect, module, ink, $kind);
            Vec::new()
        }
    };
}
env_draw!(env_level, EnvelopeKind::Level);
env_draw!(env_mod, EnvelopeKind::Mod);
env_draw!(env_eg, EnvelopeKind::Eg);

/// The `FX` page: the slot list over the selected slot's on/off pill, then the selected slot's controls. Slot k is
/// #(k + 2) with params 46 + 7k (kind) to 52 + 7k (Y).
fn fx() -> Part {
    let slot = |k: usize| Show::Setting(81100, k);
    let pill = |k: usize| {
        toggle(81200 + 100 * k as u32, "")
            .p(47 + 7 * k)
            .show(slot(k))
    };
    let page = |k: usize| {
        let (id, p) = (80000 + 1000 * (k as u32 + 2), 46 + 7 * k);
        hstack([
            vstack([
                hstack([
                    spinner(id + 111, "FX KIND")
                        .p(p)
                        .items_per_column(5)
                        .compact(),
                    knob(id + 112, "AMNT").p(p + 4),
                ]),
                hstack([
                    spinner(id + 121, "LFO RATE")
                        .p(p + 2)
                        .items_per_column(5)
                        .compact(),
                    spinner(id + 122, "LFO SHAPE").p(p + 3).compact(),
                ]),
            ])
            .w(Size::Flex(2.0)),
            knob(id + 200, "X").p(p + 5),
            knob(id + 300, "Y").p(p + 6),
        ])
        .w(Size::Flex(4.0))
        .show(slot(k))
    };
    hstack([
        vstack([
            list(81100, "1 · 2 · 3", Bar::Right).h(Size::Flex(3.0)),
            pill(0),
            pill(1),
            pill(2),
        ]),
        page(0),
        page(1),
        page(2),
    ])
}

#[cfg(test)]
const STRIP: Row = row(
    0,
    Widget::Tabs,
    "LEVEL · OSC · MOD · FILTER · EG · LFO · VOICES · FX",
    [0.0, 0.0, 16.96, 1.25],
)
.h(Size::U(1.25));
#[cfg(test)]
const SLOTS: Row = row(
    81100,
    Widget::List(Bar::Right),
    "1 · 2 · 3",
    [0.00, 1.25, 3.39, 4.68],
)
.h(Size::Flex(3.0));

/// OSC `A` and `B`: the 29 waveshapes.
#[cfg(test)]
const SHAPES: &str = "Beast · Crunch · Double Spike · Digital · Dirt · Electro · Filth · Fox · Funky · Grubby · Keys · \
    Liquid · Ogre · Rhodes · Saw Mean · Saw Retro · Saw · Silk · Sine · Smooth Sub · Square Retro · Square Smooth · \
    Square · Strat · String · Throat · Triangle · Troll · Vocal";

#[cfg(test)]
#[rustfmt::skip]
pub const REFERENCE: Reference = Reference {
    size: [16.96, 7.49],
    // A and B on Beast; PB RANGE, MW DEST and MW AMOUNT from the CSTM defaults (items 1 and 2, 0.5); FX slot 1; and
    // Honeydew's own FILTER EXTRA (16300) on Off and FM RANGE (32100) on x1.
    settings: &[(16300, 0.0), (21100, 0.0), (21200, 0.0), (32100, 0.0), (36100, 1.0), (36200, 2.0), (37000, 0.5), (81100, 0.0)],
    cases: &[
        Case {
            name: "FILTER",
            tab: 3,
            rows: &[
                STRIP,
                row(12100, Widget::Display, "", [0.10, 1.35, 16.76, 2.63]).margins(0.1, 0.1, 0.1, 0.1),
                row(11000, Widget::Knob, "CUTOFF", [0.00, 4.08, 3.39, 2.01]).p(21),
                row(12000, Widget::Knob, "RES", [3.39, 4.08, 3.39, 2.01]).p(22),
                row(13000, Widget::Knob, "ATTACK", [6.78, 4.08, 3.39, 2.01]).p(25),
                row(14000, Widget::Knob, "DECAY", [10.17, 4.08, 3.39, 2.01]).p(26),
                row(15000, Widget::Knob, "AMOUNT", [13.56, 4.08, 3.40, 2.01]).p(27),
                row(16100, Widget::TextToggle, "KBD", [0.00, 6.09, 3.11, 1.40]).w(Size::Flex(0.45)).p(24),
                row(16200, Widget::Spinner, "KIND", [3.11, 6.09, 6.92, 1.40]).p(23),
                row(16300, Widget::Spinner, "EXTRA", [10.03, 6.09, 6.93, 1.40]).items("Off · BP · Notch · Comb"),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "FILTER over OSC (two pages)",
            tab: 3,
            lower: Some(1),
            height: Some(16.0),
            rows: &[
                STRIP,
                row(12100, Widget::Display, "", [0.10, 1.35, 16.76, 2.93]).margins(0.1, 0.1, 0.1, 0.1),
                row(11000, Widget::Knob, "CUTOFF", [0.00, 4.38, 3.39, 2.22]).p(21),
                row(12000, Widget::Knob, "RES", [3.39, 4.38, 3.39, 2.22]).p(22),
                row(13000, Widget::Knob, "ATTACK", [6.78, 4.38, 3.39, 2.22]).p(25),
                row(14000, Widget::Knob, "DECAY", [10.17, 4.38, 3.39, 2.22]).p(26),
                row(15000, Widget::Knob, "AMOUNT", [13.56, 4.38, 3.40, 2.22]).p(27),
                row(16100, Widget::TextToggle, "KBD", [0.00, 6.60, 3.11, 1.40]).w(Size::Flex(0.45)).p(24),
                row(16200, Widget::Spinner, "KIND", [3.11, 6.60, 6.92, 1.40]).p(23),
                row(16300, Widget::Spinner, "EXTRA", [10.03, 6.60, 6.93, 1.40]).items("Off · BP · Notch · Comb"),
                row(1, Widget::Tabs, "LEVEL · OSC · MOD · FILTER · EG · LFO · VOICES · FX", [0.00, 8.00, 16.96, 1.25]).h(Size::U(1.25)),
                row(21100, Widget::Spinner, "A", [0.10, 9.35, 8.28, 3.57]).items(SHAPES).items_per_column(10).margins(0.1, 0.1, 0.1, 0.1),
                row(21200, Widget::Spinner, "B", [8.58, 9.35, 8.28, 3.57]).items(SHAPES).items_per_column(10).margins(0.1, 0.1, 0.1, 0.1),
                row(22000, Widget::Knob, "PITCH B", [0.00, 13.02, 3.39, 2.97]).p(11),
                row(23000, Widget::Knob, "FINE B", [3.39, 13.02, 3.39, 2.97]).p(12),
                row(24000, Widget::Knob, "PHASE B", [6.78, 13.02, 3.39, 2.97]).p(14),
                row(25000, Widget::Knob, "BALANCE", [10.17, 13.02, 3.39, 2.97]).p(13),
                row(26000, Widget::Knob, "NOISE", [13.56, 13.02, 3.40, 2.97]).p(20),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "OSC",
            tab: 1,
            rows: &[
                STRIP,
                row(21100, Widget::Spinner, "A", [0.10, 1.35, 8.28, 3.28]).items(SHAPES).items_per_column(10).margins(0.1, 0.1, 0.1, 0.1),
                row(21200, Widget::Spinner, "B", [8.58, 1.35, 8.28, 3.28]).items(SHAPES).items_per_column(10).margins(0.1, 0.1, 0.1, 0.1),
                row(22000, Widget::Knob, "PITCH B", [0.00, 4.73, 3.39, 2.76]).p(11),
                row(23000, Widget::Knob, "FINE B", [3.39, 4.73, 3.39, 2.76]).p(12),
                row(24000, Widget::Knob, "PHASE B", [6.78, 4.73, 3.39, 2.76]).p(14),
                row(25000, Widget::Knob, "BALANCE", [10.17, 4.73, 3.39, 2.76]).p(13),
                row(26000, Widget::Knob, "NOISE", [13.56, 4.73, 3.40, 2.76]).p(20),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "MOD",
            tab: 2,
            rows: &[
                STRIP,
                row(31100, Widget::Display, "", [0.10, 1.35, 16.76, 2.56]).margins(0.1, 0.1, 0.1, 0.1),
                row(31000, Widget::Knob, "MOD. AMNT", [0.00, 4.01, 3.39, 2.08]).p(15),
                row(33000, Widget::Knob, "ATTACK", [3.39, 4.01, 3.39, 2.08]).p(17),
                row(34000, Widget::Knob, "DECAY", [6.78, 4.01, 3.39, 2.08]).p(18),
                row(35000, Widget::Knob, "EG AMOUNT", [10.17, 4.01, 3.39, 2.08]).p(19),
                row(37000, Widget::Knob, "MW AMOUNT", [13.56, 4.01, 3.40, 2.08]),
                row(32000, Widget::Spinner, "KIND", [0.00, 6.09, 4.24, 1.40]).p(16),
                row(32100, Widget::Spinner, "FM RANGE", [4.24, 6.09, 4.24, 1.40]).items("x1 · x4 · x8 · x16"),
                row(36100, Widget::Spinner, "PB RANGE", [8.48, 6.09, 4.24, 1.40]).items("1 smt · 2 smt · 7 smt · Oct"),
                row(36200, Widget::Spinner, "MW DEST", [12.72, 6.09, 4.24, 1.40]).items("Filter · LFO · Osc. Mod."),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "LEVEL",
            tab: 0,
            rows: &[
                STRIP,
                row(41100, Widget::Display, "", [0.10, 1.35, 16.76, 3.51]).margins(0.1, 0.1, 0.1, 0.1),
                row(41000, Widget::Knob, "LEVEL", [0.00, 4.96, 2.82, 2.53]).p(1),
                row(42000, Widget::Knob, "PAN", [2.82, 4.96, 2.83, 2.53]).p(2),
                row(43000, Widget::Knob, "ATK", [5.65, 4.96, 2.83, 2.53]).p(3),
                row(44000, Widget::Knob, "DEC", [8.48, 4.96, 2.82, 2.53]).p(4),
                row(45000, Widget::Knob, "SUS", [11.30, 4.96, 2.83, 2.53]).p(5),
                row(46000, Widget::Knob, "REL", [14.13, 4.96, 2.82, 2.53]).p(6),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "EG",
            tab: 4,
            rows: &[
                STRIP,
                row(51100, Widget::Display, "", [0.10, 1.35, 16.76, 2.56]).margins(0.1, 0.1, 0.1, 0.1),
                row(51000, Widget::Knob, "AMOUNT", [0.00, 4.01, 3.39, 2.08]).p(40),
                row(52000, Widget::Knob, "ATTACK", [3.39, 4.01, 3.39, 2.08]).p(36),
                row(53000, Widget::Knob, "DECAY", [6.78, 4.01, 3.39, 2.08]).p(37),
                row(54000, Widget::Knob, "SUSTAIN", [10.17, 4.01, 3.39, 2.08]).p(38),
                row(55000, Widget::Knob, "RELEASE", [13.56, 4.01, 3.40, 2.08]).p(39),
                row(56100, Widget::TextToggle, "VEL", [0.00, 6.09, 5.26, 1.40]).w(Size::Flex(0.45)).p(74),
                row(56200, Widget::Spinner, "DEST", [5.26, 6.09, 11.70, 1.40]).p(41),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "LFO",
            tab: 5,
            rows: &[
                STRIP,
                row(61100, Widget::Display, "", [0.10, 1.35, 16.76, 3.72]).margins(0.1, 0.1, 0.1, 0.1),
                row(61000, Widget::Knob, "AMOUNT", [0.00, 5.17, 4.24, 2.32]).p(34),
                row(62000, Widget::Spinner, "SHAPE", [4.24, 5.17, 4.24, 2.32]).p(33),
                row(63000, Widget::Spinner, "DEST", [8.48, 5.17, 4.24, 2.32]).p(35),
                row(64000, Widget::Spinner, "RATE", [12.72, 5.17, 4.24, 2.32]).p(32).items_per_column(5),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "VOICES",
            tab: 6,
            rows: &[
                STRIP,
                row(71000, Widget::Knob, "TRNSP", [0.00, 1.25, 4.24, 4.84]).p(28),
                row(72000, Widget::Knob, "SLIDE", [4.24, 1.25, 4.24, 4.84]).p(29),
                row(73000, Widget::Knob, "CHOIR DTN", [8.48, 1.25, 4.24, 4.84]).p(44),
                row(74000, Widget::Knob, "CHOIR STR", [12.72, 1.25, 4.24, 4.84]).p(45),
                row(75100, Widget::TextToggle, "MONO", [0.00, 6.09, 3.18, 1.40]).w(Size::Flex(0.6)).p(30),
                row(75200, Widget::TextToggle, "AUTO", [3.18, 6.09, 3.18, 1.40]).w(Size::Flex(0.6)).p(31),
                row(76100, Widget::Spinner, "VOICES", [6.36, 6.09, 5.30, 1.40]).p(42),
                row(76200, Widget::Spinner, "SCALE", [11.66, 6.09, 5.30, 1.40]).p(43),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "FX slot 1",
            tab: 7,
            rows: &[
                STRIP,
                SLOTS,
                row(81200, Widget::Toggle, "", [0.00, 5.93, 3.39, 1.56]).p(47),
                row(82111, Widget::Spinner, "FX KIND", [3.39, 1.25, 3.39, 3.12]).p(46).items_per_column(5),
                row(82112, Widget::Knob, "AMNT", [6.78, 1.25, 3.39, 3.12]).p(50),
                row(82121, Widget::Spinner, "LFO RATE", [3.39, 4.37, 3.39, 3.12]).p(48).items_per_column(5),
                row(82122, Widget::Spinner, "LFO SHAPE", [6.78, 4.37, 3.39, 3.12]).p(49),
                row(82200, Widget::Knob, "X", [10.17, 1.25, 3.39, 6.24]).p(51),
                row(82300, Widget::Knob, "Y", [13.56, 1.25, 3.40, 6.24]).p(52),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "FX slot 2",
            tab: 7,
            settings: &[(81100, 1.0)],
            rows: &[
                STRIP,
                SLOTS,
                row(81300, Widget::Toggle, "", [0.00, 5.93, 3.39, 1.56]).p(54),
                row(83111, Widget::Spinner, "FX KIND", [3.39, 1.25, 3.39, 3.12]).p(53).items_per_column(5),
                row(83112, Widget::Knob, "AMNT", [6.78, 1.25, 3.39, 3.12]).p(57),
                row(83121, Widget::Spinner, "LFO RATE", [3.39, 4.37, 3.39, 3.12]).p(55).items_per_column(5),
                row(83122, Widget::Spinner, "LFO SHAPE", [6.78, 4.37, 3.39, 3.12]).p(56),
                row(83200, Widget::Knob, "X", [10.17, 1.25, 3.39, 6.24]).p(58),
                row(83300, Widget::Knob, "Y", [13.56, 1.25, 3.40, 6.24]).p(59),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "FX slot 3",
            tab: 7,
            settings: &[(81100, 2.0)],
            rows: &[
                STRIP,
                SLOTS,
                row(81400, Widget::Toggle, "", [0.00, 5.93, 3.39, 1.56]).p(61),
                row(84111, Widget::Spinner, "FX KIND", [3.39, 1.25, 3.39, 3.12]).p(60).items_per_column(5),
                row(84112, Widget::Knob, "AMNT", [6.78, 1.25, 3.39, 3.12]).p(64),
                row(84121, Widget::Spinner, "LFO RATE", [3.39, 4.37, 3.39, 3.12]).p(62).items_per_column(5),
                row(84122, Widget::Spinner, "LFO SHAPE", [6.78, 4.37, 3.39, 3.12]).p(63),
                row(84200, Widget::Knob, "X", [10.17, 1.25, 3.39, 6.24]).p(65),
                row(84300, Widget::Knob, "Y", [13.56, 1.25, 3.40, 6.24]).p(66),
            ],
            ..Case::DEFAULT
        },
    ],
};

#[cfg(test)]
mod curve_tests {
    use super::*;
    use honeydew_audio::gm_waveshapes::{SAW, SINE};

    fn cycles(a: usize, b: usize) -> (Vec<f32>, Vec<f32>) {
        let get = |slot| honeydew_audio::gmsynth::preview_cycle(slot, DISPLAY_HARMONICS).unwrap();
        (get(a), get(b))
    }

    fn module(kind: f32, amount: f32) -> ModuleInstance {
        let mut module = ModuleInstance::new(2, 1);
        module.params[16] = kind;
        module.params[15] = amount;
        module
    }

    #[test]
    fn a_with_no_modulation_is_its_plain_shape_and_each_kind_changes_it() {
        let (a, b) = cycles(SAW, SINE);
        let state = PanelState::default();
        let plain = curve_a(&a, &b, &module(0.4, 0.0), &state, 200);
        assert_eq!(plain.len(), 201);
        // At 0 amount ring and FM leave A alone, whatever the KIND.
        assert_eq!(plain, curve_a(&a, &b, &module(0.0, 0.0), &state, 200));
        for (kind, name) in [(0.0, "ring"), (0.4, "FM")] {
            let modulated = curve_a(&a, &b, &module(kind, 0.5), &state, 200);
            assert!(modulated != plain, "{name} changes A");
            assert!(modulated.iter().all(|v| v.is_finite()), "{name}");
        }
    }

    #[test]
    fn a_larger_fm_range_bends_a_further_at_the_same_amount() {
        let (a, b) = cycles(SAW, SINE);
        let module = module(0.4, 0.2);
        let mut state = PanelState::default();
        let curve = |state: &PanelState| curve_a(&a, &b, &module, state, 200);
        let x1 = curve(&state);
        state.settings.insert(32100, 2.0);
        let x8 = curve(&state);
        assert!(x1 != x8);
        // Item 0 is the original depth.
        state.settings.insert(32100, 0.0);
        assert_eq!(curve(&state), x1);
    }

    #[test]
    fn sync_restarts_a_each_time_b_wraps() {
        let (a, b) = cycles(SAW, SINE);
        // PITCH B an octave up (v11 = 0.75): B wraps twice per A cycle, so A shows two half cycles.
        let mut module = module(0.7, 0.5);
        module.params[11] = 0.75;
        let curve = curve_a(&a, &b, &module, &PanelState::default(), 400);
        let (first, second) = (curve[0], curve[200]);
        assert!(
            (first - second).abs() < 0.05,
            "A starts over at B's wrap: {first} {second}"
        );
    }

    #[test]
    fn dragging_the_filter_display_sets_the_cutoff_and_the_resonance() {
        use crate::metrics::{DEFAULT_UI_SCALE, Metrics};
        use eframe::egui::{CentralPanel, Context, Event, PointerButton, RawInput, vec2};
        let ctx = Context::default();
        let rect = Rect::from_min_size(pos2(100.0, 100.0), vec2(400.0, 150.0));
        let mut module = ModuleInstance::new(2, 1);
        let mut state = PanelState::default();
        let (theme, m) = (
            crate::theme::theme_by_id(0),
            Metrics::from_pixels_per_point(1.0, DEFAULT_UI_SCALE),
        );
        let cx = UiCtx {
            theme: &theme,
            m: &m,
        };
        let mut changed = Vec::new();
        let mut run = |events: Vec<Event>, module: &mut ModuleInstance, time: f64| {
            let raw = RawInput {
                time: Some(time),
                events,
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0))),
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                CentralPanel::default().show(ctx, |ui| {
                    let ink = Color32::RED;
                    changed.extend(filter_display(
                        ui,
                        &cx,
                        Id::new("filter"),
                        rect,
                        module,
                        &mut state,
                        ink,
                    ));
                });
            });
        };
        let start = rect.center();
        run(vec![], &mut module, 0.0);
        run(
            vec![
                Event::PointerMoved(start),
                Event::PointerButton {
                    pos: start,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            &mut module,
            0.1,
        );
        // Drag to the middle of the display's width, a quarter of the way down from the top.
        let to = pos2(
            rect.left() + rect.width() * 0.5,
            rect.top() + rect.height() * 0.25,
        );
        run(vec![Event::PointerMoved(to)], &mut module, 0.2);
        run(
            vec![Event::PointerMoved(to + vec2(1.0, 0.0))],
            &mut module,
            0.3,
        );
        assert!(
            changed.contains(&21) && changed.contains(&22),
            "{changed:?}"
        );
        // The middle of a 20 Hz – 20 kHz axis is about 632 Hz; the resonance is high near the top.
        let hz = honeydew_audio::gmsynth::cutoff_hz(module.params[21]);
        assert!((400.0..900.0).contains(&hz), "{hz} Hz");
        assert!(module.params[22] > 0.6, "{}", module.params[22]);
    }
}
