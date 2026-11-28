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
use crate::theme::Accent;
use crate::ui::{UiCtx, widgets};
// The waveshapes of the OSC `A` and `B` spinners, kept with the preset, not as params.
use crate::params::GMSYNTH_SOURCES as WAVESHAPES;

const PB_RANGE: &[&str] = &["1 smt", "2 smt", "7 smt", "Oct"];
const MW_DEST: &[&str] = &["Filter", "LFO", "Osc. Mod."];
/// Honeydew's FM depth range, a `CSTM` setting; `x1` is the original depth.
const FM_RANGES: &[&str] = &["x1", "x4", "x8", "x16"];
/// Honeydew's LFO modes, a `CSTM` setting: the original shapes, sample and hold, a new random value per note.
const LFO_MODES: &[&str] = &["Shape", "S&H", "Note"];
/// Honeydew's LFO note values, a `CSTM` setting: the cycle in notes, triplets and dotted notes; `Orig.` keeps the RATE menu's.
const LFO_SYNCS: &[&str] = &[
    "Orig.", "1/32", "1/16T", "1/16", "1/8T", "1/16.", "1/8", "1/4T", "1/8.", "1/4", "1/2",
    "1 Bar", "2 Bars", "4 Bars", "8 Bars",
];
/// The LFO is in its original mode (the shape menu's).
static LFO_SHAPED: Show = Show::Setting(65100, 0);
/// Honeydew's extra filter types, a `CSTM` setting; `Off` keeps the `KIND` spinner's filter.
const FILTER_EXTRA: &[&str] = &["Off", "BP", "Notch", "Comb"];

/// The oscillator menu as submenus (a tap on A or B lists these, a group opens its items): the 29 classic shapes in four groups
/// by what they sound like, then the built-in wavetables in three. Only an arrangement: each item keeps its index in the menu,
/// which the song stores, so a group lists indices of [`WAVESHAPES`].
const SOURCE_GROUPS: &[widgets::Group] = &[
    ("Basic", &[18, 26, 16, 22, 19, 17, 15, 14, 20, 21]),
    ("Keys & Strings", &[10, 13, 23, 24, 11]),
    ("Voices", &[28, 25, 12, 27]),
    ("Rough & Digital", &[3, 1, 4, 5, 6, 9, 0, 2, 7, 8]),
    ("Table: Morph", &[29, 30, 31, 33, 36, 40]),
    ("Table: Formant & Bell", &[32, 39]),
    ("Table: Digital", &[34, 35, 37, 38]),
    ("Custom file", &[41]),
];

/// The colour of each page's tab, in the tabs' order: the selected tab shows its page's colour, the others pale.
const TAB_ACCENTS: &[Accent] = &[
    Accent::Level,
    Accent::Osc,
    Accent::Mod,
    Accent::Filter,
    Accent::Envelope,
    Accent::Lfo,
    Accent::Lfo,
    Accent::Level,
    Accent::Fx,
];

const TABS: &str = "LEVEL · OSC · MOD · FILTER · EG · LFO · MATRIX · VOICES · FX";

/// The sources and targets of Honeydew's free modulation routes (a `CSTM` setting each, see
/// [`honeydew_audio::gmsynth::MOD_BASE`]); they must match the engine's lists.
const MOD_SOURCES: &[&str] = &["Off", "LFO", "EG"];
const MOD_TARGETS: &[&str] = &[
    "Off",
    "Level",
    "Pan",
    "Osc B Pitch",
    "Osc B Detune",
    "Balance",
    "Mod Amount",
    "Noise",
    "Cutoff",
    "Resonance",
    "Filter EG",
    "EG Amount",
    "LFO Amount",
    "Choir Detune",
    "Choir Stereo",
    "FX 1 X",
    "FX 1 Y",
    "FX 2 X",
    "Table Pos A",
    "Table Pos B",
];

/// One route of the MATRIX page: its source and target menus stacked beside a big bipolar amount knob (centre = 0).
fn route(slot: u32) -> Part {
    let id = 66000 + 10 * slot;
    line([
        page([
            spinner(id, "FROM").items(MOD_SOURCES).compact(),
            spinner(id + 1, "TO")
                .items(MOD_TARGETS)
                .items_per_column(6)
                .compact(),
        ])
        .w(Size::Flex(1.25)),
        knob(id + 2, "AMOUNT").default(0.5).w(Size::Flex(1.0)),
    ])
    .margins(0.06, 0.06, 0.06, 0.06)
}

/// A page of rows of controls, top to bottom.
fn page(rows: impl IntoIterator<Item = Part>) -> Part {
    vstack(rows)
}

/// A row of controls, left to right.
fn line(parts: impl IntoIterator<Item = Part>) -> Part {
    hstack(parts)
}

/// A knob row's share of a page's height, against its picture's.
fn knobs(parts: impl IntoIterator<Item = Part>, share: f32) -> Part {
    line(parts).h(Size::Flex(share))
}

/// A picture of what the page's controls do, in a well of its own.
fn picture(id: u32, draw: crate::panels::Draw, share: f32) -> Part {
    line([display(id).draw(draw).margins(0.1, 0.1, 0.1, 0.1)]).h(Size::Flex(share))
}

/// A page's main part with its menus and switches stacked at the right edge, in a column a quarter as wide: they are
/// used less than the knobs, so the knobs and the picture keep the room.
fn with_side(main: Part, side: impl IntoIterator<Item = Part>) -> Part {
    let gap = |part: Part| part.margins(0.06, 0.0, 0.06, 0.0);
    line([
        main.w(Size::Flex(3.6)),
        page(side.into_iter().map(gap)).w(Size::Flex(1.0)),
    ])
}

pub fn panel() -> Option<Panel> {
    // Pages under two tab strips: with two pages at once (a tall portrait card) each has a strip of its own. A picture
    // on top of what it shows, the knobs under it (input below, as on a phone's keyboard), and the menus and switches
    // stacked at the right edge. Each page has its accent ([`Accent`]).
    Some(Panel::new(pager(
        tabs(0, TABS).accents(TAB_ACCENTS).h(Size::U(1.25)),
        tabs(LOWER_TABS, TABS).accents(TAB_ACCENTS).h(Size::U(1.25)),
        vec![
            // LEVEL
            tint(
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
                Accent::Level,
            ),
            // OSC: each oscillator is its own menu, a live picture of the wave whose tap opens the shape list.
            tint(
                page([
                    line([
                        spinner(21100, "A")
                            .items(WAVESHAPES.as_slice())
                            .groups(SOURCE_GROUPS)
                            .items_per_column(10)
                            .draw(wave_a)
                            .margins(0.1, 0.1, 0.1, 0.1),
                        spinner(21200, "B")
                            .items(WAVESHAPES.as_slice())
                            .groups(SOURCE_GROUPS)
                            .items_per_column(10)
                            .draw(wave_b)
                            .margins(0.1, 0.1, 0.1, 0.1),
                    ])
                    .h(Size::Flex(1.9)),
                    // Honeydew's own position knobs sit under their oscillator's picture: where its wavetable is read.
                    knobs(
                        [knob(27000, "POS A").p(75), knob(28000, "POS B").p(76)],
                        1.0,
                    ),
                    knobs(
                        [
                            knob(22000, "PITCH B").p(11),
                            knob(23000, "FINE B").p(12),
                            knob(24000, "PHASE B").p(14),
                            knob(25000, "BALANCE").p(13),
                            knob(26000, "NOISE").p(20),
                        ],
                        1.3,
                    ),
                ]),
                Accent::Osc,
            ),
            // MOD
            tint(
                with_side(
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
                    ]),
                    [
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
                    ],
                ),
                Accent::Mod,
            ),
            // FILTER
            tint(
                with_side(
                    page([
                        picture(12100, filter_display, 2.4),
                        knobs(
                            [
                                knob(11000, "CUTOFF").p(21),
                                knob(12000, "RES").p(22),
                                knob(13000, "ATTACK").p(25),
                                knob(14000, "DECAY").p(26),
                                knob(15000, "AMOUNT").p(27),
                                // Honeydew's own DRIVE: pushes the sound into the filter's soft clip.
                                knob(16400, "DRIVE").default(0.0),
                            ],
                            1.7,
                        ),
                    ]),
                    [
                        text_toggle(16100, "KBD").p(24),
                        spinner(16200, "KIND").p(23).compact(),
                        // Honeydew's own extra filter types.
                        spinner(16300, "EXTRA").items(FILTER_EXTRA).compact(),
                    ],
                ),
                Accent::Filter,
            ),
            // EG
            tint(
                with_side(
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
                    ]),
                    [
                        text_toggle(56100, "VEL").p(74),
                        spinner(56200, "DEST").p(41).compact(),
                    ],
                ),
                Accent::Envelope,
            ),
            // LFO
            tint(
                with_side(
                    page([
                        picture(61100, lfo_display, 1.6),
                        // AMOUNT at the foot, with the random range beside it while the LFO is random (S&H or Note).
                        knobs(
                            [
                                knob(61000, "AMOUNT").p(34),
                                // A note restarts the LFO's cycle.
                                text_toggle(65500, "KEY").w(Size::Flex(0.5)),
                                knob(65200, "LOW").default(0.0).show(Show::Not(&LFO_SHAPED)),
                                knob(65300, "HIGH")
                                    .default(1.0)
                                    .show(Show::Not(&LFO_SHAPED)),
                            ],
                            1.0,
                        ),
                    ]),
                    [
                        spinner(65100, "MODE").items(LFO_MODES).compact(),
                        spinner(62000, "SHAPE").p(33).compact(),
                        spinner(63000, "DEST").p(35).compact(),
                        spinner(64000, "RATE").p(32).items_per_column(5).compact(),
                        // Honeydew's note values, beside the original RATE; `Orig.` keeps RATE.
                        spinner(65400, "SYNC")
                            .items(LFO_SYNCS)
                            .items_per_column(8)
                            .compact(),
                    ],
                ),
                Accent::Lfo,
            ),
            // MATRIX: free routes of the LFO or the EG to any listed knob.
            tint(
                page([
                    // What the LFO and EG pages' own routes do, which keep working whatever the routes below say.
                    display(66500).draw(page_routes_note).h(Size::U(0.9)),
                    line([route(0), route(1)]),
                    line([route(2), route(3)]),
                ]),
                Accent::Lfo,
            ),
            // VOICES
            tint(
                with_side(
                    knobs(
                        [
                            knob(71000, "TRNSP").p(28),
                            knob(72000, "SLIDE").p(29),
                            knob(73000, "CHOIR DTN").p(44),
                            knob(74000, "CHOIR STR").p(45),
                        ],
                        1.0,
                    ),
                    [
                        text_toggle(75100, "MONO").p(30),
                        text_toggle(75200, "AUTO").p(31),
                        spinner(76100, "VOICES").p(42).compact(),
                        spinner(76200, "SCALE").p(43).compact(),
                    ],
                ),
                Accent::Level,
            ),
            tint(fx(), Accent::Fx),
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
/// The cycles the LFO display shows.
const LFO_CYCLES: f32 = 2.0;
/// The most cycles of A that A's picture spans, when a slow B modulates it.
const MAX_A_CYCLES: f64 = 4.0;

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
    // How many cycles of A the picture spans. One, unless B modulates A and is slower: B then runs over several cycles of
    // A (that is the sound of a low modulator: a slow wobble, not a changed shape), so the window opens to as many cycles
    // as B's cycle lasts, at most [`MAX_A_CYCLES`], and the picture shows A's cycles changing.
    let modulated = match kind {
        0 | 1 => amount > 0.001,
        2 => true,
        _ => false,
    };
    let cycles_shown = if modulated {
        (1.0 / ratio).clamp(1.0, MAX_A_CYCLES)
    } else {
        1.0
    };
    // B is read around the middle of the window, so that its influence is centred on the picture and a slow B does not
    // slide in from one side.
    let phase_b_at = |t: f64| (t - cycles_shown / 2.0) * ratio + offset;
    let mut wraps = phase_b_at(0.0).floor() as i64;
    (0..=steps)
        .map(|i| {
            let t = cycles_shown * i as f64 / steps as f64;
            let phase_b = phase_b_at(t);
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
    // An oscillator's table when it plays a wavetable: a built-in one, or the file its module took ("Custom").
    let table_of = |key: u32| -> Option<std::sync::Arc<honeydew_audio::wavetable::FrameTable>> {
        let item = *state.settings.get(&key)? as usize;
        if item == honeydew_audio::gmsynth::CUSTOM_ITEM {
            let data = module.file.tables[usize::from(key == 21200)].as_ref()?;
            return honeydew_audio::gmsynth::custom_table(data.key, || {
                honeydew_audio::wavetable::FrameTable::from_cycles(&data.cycles())
            });
        }
        honeydew_audio::gmsynth::factory_table(item)
    };
    // One cycle of an oscillator as it is read at its position: a classic shape or a table.
    let shape = |key: u32| {
        let slot = *state.settings.get(&key)?;
        let pos = param(if key == 21200 { 76 } else { 75 }).clamp(0.0, 1.0);
        match table_of(key) {
            Some(table) => Some(honeydew_audio::gmsynth::table_cycle(&table, pos)),
            None => honeydew_audio::gmsynth::source_cycle(slot as usize, DISPLAY_HARMONICS, pos),
        }
    };
    let slot = state
        .settings
        .get(&if b { 21200 } else { 21100 })
        .map(|&v| v as usize);
    let own_key = if b { 21200 } else { 21100 };
    // `Custom` with no file is drawn empty and says so; the other oscillator may be anything.
    let custom_name = (slot == Some(honeydew_audio::gmsynth::CUSTOM_ITEM)).then(|| {
        module.file.tables[usize::from(b)]
            .as_ref()
            .map_or("no file yet".to_owned(), |t| t.name.clone())
    });
    let (Some(own), Some(other)) = (shape(own_key), shape(if b { 21100 } else { 21200 })) else {
        if custom_name.is_some() {
            caption(
                ui,
                cx,
                inner,
                if b { "B" } else { "A" },
                slot,
                ink,
                custom_name.as_deref(),
            );
        }
        return;
    };
    // BALANCE: A takes 1 − |2v − 1| of it, B 2v − 1 (negative: B upside down).
    let balance = 2.0 * param(13) - 1.0;
    // A wavetable is drawn as its stack of frames in perspective, the oscillator's own frame in front of them.
    if let Some(table) = table_of(own_key) {
        let position = param(if b { 76 } else { 75 });
        let share = if b {
            balance.abs()
        } else {
            1.0 - balance.abs()
        };
        let ink = ink.gamma_multiply(0.25 + 0.75 * share);
        ridge(ui, cx, inner, &table, position, &own, ink);
        caption(
            ui,
            cx,
            inner,
            if b { "B" } else { "A" },
            slot,
            ink,
            custom_name.as_deref(),
        );
        return;
    }
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
    caption(
        ui,
        cx,
        inner,
        if b { "B" } else { "A" },
        slot,
        ink,
        custom_name.as_deref(),
    );
}

/// How many of a wavetable's frames the 3D picture stacks, at most.
const RIDGE_LINES: usize = 14;
/// Points per line of the 3D picture.
const RIDGE_POINTS: usize = 72;

/// A wavetable's 3D picture in `inner`: its frames (at most [`RIDGE_LINES`], evenly spread) as lines stacked in
/// perspective, the first in front at the bottom and the last at the back, each higher, smaller and fainter than the one
/// before; and the oscillator's own frame, `own` (one cycle at the `position`), as a bright line with a soft fill at its
/// depth. The oscillator reads it there, so the picture moves with the POS knob, its automation and its modulation.
fn ridge(
    ui: &Ui,
    cx: &UiCtx,
    inner: Rect,
    table: &honeydew_audio::wavetable::FrameTable,
    position: f32,
    own: &[f32],
    ink: Color32,
) {
    let frames = table.frames();
    let t = cx.m.t();
    let painter = ui.painter();
    let (depth_x, depth_y) = (inner.width() * 0.14, inner.height() * 0.40);
    let base_w = inner.width() - depth_x - 2.0 * t;
    let floor = inner.bottom() - inner.height() * 0.10;
    // Where depth `z` (0 front, 1 back) puts a point at `u` across the cycle with `v` of the line's height.
    let amp0 = inner.height() * 0.24;
    let place = |z: f32, u: f32, v: f32| {
        pos2(
            inner.left() + t + depth_x * z + base_w * u * (1.0 - 0.15 * z),
            floor - depth_y * z - amp0 * (1.0 - 0.3 * z) * v,
        )
    };
    let lines = frames.min(RIDGE_LINES);
    let outlines: Vec<Vec<f32>> = (0..lines)
        .map(|k| {
            let frame = (k * (frames - 1) + (lines - 1) / 2) / (lines - 1).max(1);
            table.outline(frame.min(frames - 1), RIDGE_POINTS)
        })
        .collect();
    let peak = (outlines.iter().flatten())
        .chain(own)
        .fold(1e-6_f32, |m, v| m.max(v.abs()));
    let line = |z: f32, values: &mut dyn Iterator<Item = f32>| -> Vec<Pos2> {
        let values: Vec<f32> = values.collect();
        let n = (values.len() - 1).max(1) as f32;
        (values.iter().enumerate())
            .map(|(i, v)| place(z, i as f32 / n, v / peak))
            .collect()
    };
    // The frames from the back to the front, each fainter the further back.
    for (k, outline) in outlines.iter().enumerate().rev() {
        let z = k as f32 / (lines - 1).max(1) as f32;
        let mut values = outline.iter().copied().chain(outline.first().copied());
        let points = line(z, &mut values);
        painter.add(Shape::line(
            points,
            Stroke::new(t * 0.8, ink.gamma_multiply(0.14 + 0.36 * (1.0 - z))),
        ));
    }
    // The oscillator's own frame at the position's depth.
    let z = if position.is_finite() {
        position.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let n = own.len().max(2);
    let step = (n / RIDGE_POINTS).max(1);
    let mut values = (0..=RIDGE_POINTS).map(|i| own[(i * step) % n]);
    let points = line(z, &mut values);
    let mut mesh = Mesh::default();
    let fill = ink.gamma_multiply(0.22);
    let ground = |p: Pos2| pos2(p.x, floor - depth_y * z);
    for pair in points.windows(2) {
        let base = mesh.vertices.len() as u32;
        for corner in [ground(pair[0]), pair[0], pair[1], ground(pair[1])] {
            mesh.colored_vertex(corner, fill);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    painter.add(Shape::mesh(mesh));
    painter.add(Shape::line(points, Stroke::new(t * 1.6, ink)));
}

/// The MATRIX page's note on the LFO and EG pages' own routes (their DEST and AMOUNT): they are not routes of this page and run
/// with all of them off, so the page names each that is on, e.g. `LFO page: Cutoff +40 %`; empty when neither is.
fn page_routes_note(
    ui: &mut Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    _: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let notes = page_routes_text(&module.params);
    if notes.is_empty() {
        return Vec::new();
    }
    ui.painter().text(
        rect.left_center() + vec2(cx.m.t() * 2.0, 0.0),
        Align2::LEFT_CENTER,
        notes,
        cx.m.font_id(FontSize::Pt8),
        ink.gamma_multiply(0.9),
    );
    Vec::new()
}

/// `LFO page: Cutoff +40 %   EG page: Mod Amount -25 %` for the page routes that are on (empty for none).
fn page_routes_text(params: &[f32]) -> String {
    use crate::panels::modulation::page_range;
    let named = [(21, "Cutoff"), (15, "Mod Amount"), (1, "Level"), (2, "Pan")];
    let mut out = Vec::new();
    for (source, label) in [(1, "LFO page"), (2, "EG page")] {
        for (knob, name) in named {
            if let Some(range) = page_range(params, knob, source) {
                out.push(format!("{label}: {name} {:+.0} %", 100.0 * range));
            }
        }
    }
    out.join("   ")
}

/// The display's caption in its top left corner, the oscillator and its shape's name, and a small chevron at the top
/// right that says a tap opens the list.
fn caption(
    ui: &Ui,
    cx: &UiCtx,
    inner: Rect,
    osc: &str,
    slot: Option<usize>,
    ink: Color32,
    custom: Option<&str>,
) {
    let painter = ui.painter();
    let m = cx.m;
    // A custom table is named by its file (or says it has none yet).
    let name = custom
        .map(|file| format!("Custom: {file}"))
        .or_else(|| {
            slot.and_then(|i| WAVESHAPES.get(i))
                .map(|s| (*s).to_owned())
        })
        .unwrap_or_default();
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
    // The live values of the synth: its spectrum, and the cutoff its voices have now.
    let live = crate::panels::live_analysis_fast(ui, module.instance_id)
        .filter(|slab| slab.len() >= honeydew_audio::gmsynth::LIVE_LEN);
    // The spectrum of what the synth is playing, behind the response: one bar per band on the same log axis, so that the
    // filter's shape is seen in the sound it shapes. Each bar has a brighter cap, and a hair of space beside it.
    if let Some(slab) = &live {
        let Some(bands) = honeydew_audio::gmsynth::live_bands(slab) else {
            return changed;
        };
        let edge = |k: usize| {
            FILTER_HZ.0 * (FILTER_HZ.1 / FILTER_HZ.0).powf(k as f32 / bands.len() as f32)
        };
        let painter = ui.painter();
        for (k, level) in bands.iter().enumerate() {
            let (left, right) = (x_of(edge(k)), x_of(edge(k + 1)));
            let height = level * inner.height() * 0.92;
            if height < t * 0.5 {
                continue;
            }
            let bar = Rect::from_min_max(
                pos2(left + t * 0.25, inner.bottom() - height),
                pos2((right - t * 0.25).max(left + t * 0.5), inner.bottom()),
            );
            painter.rect_filled(bar, 0.0, text.gamma_multiply(0.16));
            let cap =
                Rect::from_min_max(bar.min, pos2(bar.max.x, bar.min.y + (t * 1.25).min(height)));
            painter.rect_filled(cap, 0.0, text.gamma_multiply(0.55));
        }
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
    // The filter as it is at this moment, with the envelopes, the LFO and the key tracking on the cutoff: it moves while
    // the knobs' own setting (the curve above) stays.
    if let Some(slab) = &live {
        let now = slab[honeydew_audio::gmsynth::LIVE_CUTOFF].clamp(0.0, 1.0);
        if (now - cutoff).abs() > 0.002 {
            let gains = honeydew_audio::gmsynth::filter_response(now, res, kind, extra, &freqs);
            let moving: Vec<Pos2> = (freqs.iter().zip(&gains))
                .map(|(&hz, &db)| pos2(x_of(hz), y_of(db)))
                .collect();
            // Only the curve moves, in the filter's own colour: the vertical cutoff line above stays at the knob.
            let painter = ui.painter();
            painter.add(Shape::line(moving, Stroke::new(t * 1.5, ink)));
        }
    }
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
    // Room above for the line of times, and a little under the curve's foot and to its sides.
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
    // The times in one line at the top left, so that short stages, which sit close together, never write over each other.
    let label = if preview.releases {
        format!(
            "A {}   D {}   S {:.0}%   R {}",
            seconds_text(preview.seconds[0]),
            seconds_text(preview.seconds[1]),
            preview.sustain * 100.0,
            seconds_text(preview.seconds[2]),
        )
    } else {
        format!(
            "A {}   D {}",
            seconds_text(preview.seconds[0]),
            seconds_text(preview.seconds[1]),
        )
    };
    painter.text(
        pos2(inner.left() + 2.0 * t, inner.top() + t),
        Align2::LEFT_TOP,
        label,
        cx.m.font_id(FontSize::Pt8),
        text.gamma_multiply(0.6),
    );
    fill_curve(ui, points, bottom, ink, t * 1.25);
}

/// The LFO display. In the original mode: [`LFO_CYCLES`] cycles of its shape, as tall as its AMOUNT (below the middle line
/// when the amount is negative). In `S&H`: a staircase of random steps (a stand-in for the values it will take, drawn
/// between the range's ends), in `Note`: one held level; both over a faint band for the range LOW … HIGH. A running dot
/// shows where the LFO is now, at the value it really has.
fn lfo_display(
    ui: &mut Ui,
    cx: &UiCtx,
    _: Id,
    rect: Rect,
    module: &mut ModuleInstance,
    state: &mut PanelState,
    ink: Color32,
) -> Vec<usize> {
    let Some(inner) = well(ui, cx, rect) else {
        return Vec::new();
    };
    let t = cx.m.t();
    let param = |p: usize| module.params.get(p).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    let setting = |key: u32, default: f32| state.settings.get(&key).copied().unwrap_or(default);
    let mode = (setting(65100, 0.0) as usize).min(2);
    let (low, high) = (
        setting(65200, 0.0).clamp(0.0, 1.0),
        setting(65300, 1.0).clamp(0.0, 1.0),
    );
    let (low, high) = (low.min(high), low.max(high));
    let amount = 2.0 * param(34) - 1.0;
    let mid = inner.center().y;
    let amp = inner.height() / 2.0 * 0.85 * amount;
    let x_at = |f: f32| inner.left() + inner.width() * f;
    let painter = ui.painter();
    painter.line_segment(
        [pos2(inner.left(), mid), pos2(inner.right(), mid)],
        Stroke::new(
            t * 0.5,
            cx.theme.colors.text.to_color32().gamma_multiply(0.15),
        ),
    );
    let points: Vec<Pos2> = match mode {
        0 => {
            let values = honeydew_audio::gmsynth::lfo_preview(
                param(33),
                LFO_CYCLES,
                curve_points(cx, inner),
            );
            let last = (values.len() - 1) as f32;
            (values.iter().enumerate())
                .map(|(i, v)| pos2(x_at(i as f32 / last), mid - v * amp))
                .collect()
        }
        _ => {
            // The range the random values lie in, as a band; and what a run of them could look like in it.
            let (top, bottom) = (
                mid - (-1.0 + 2.0 * high) * amp,
                mid - (-1.0 + 2.0 * low) * amp,
            );
            let band = Rect::from_x_y_ranges(inner.x_range(), top.min(bottom)..=top.max(bottom));
            painter.rect_filled(band, 0.0, ink.gamma_multiply(0.12));
            let steps = if mode == 1 { 8 } else { 1 };
            // A fixed pseudo-random run (a golden-ratio sequence), the same every time the picture is drawn.
            let value_at = |k: usize| {
                let u = (0.37 + k as f32 * 0.618_034).fract();
                -1.0 + 2.0 * (low + (high - low) * u)
            };
            let mut points = Vec::new();
            for k in 0..steps {
                let y = mid - value_at(k + 1) * amp;
                points.push(pos2(x_at(k as f32 / steps as f32), y));
                points.push(pos2(x_at((k + 1) as f32 / steps as f32), y));
            }
            points
        }
    };
    let base = if mode == 0 { mid } else { inner.bottom() };
    fill_curve(ui, points, base, ink, t * 1.25);
    // The running dot, at the value the LFO has now. In the original mode on its curve, in each cycle shown; in the
    // random modes on a line across the picture at the held value, moving with the phase.
    if let Some(slab) = crate::panels::live_analysis_fast(ui, module.instance_id)
        && slab.len() >= honeydew_audio::gmsynth::LIVE_LEN
    {
        let (value, phase) = (
            slab[honeydew_audio::gmsynth::LIVE_LFO],
            slab[honeydew_audio::gmsynth::LIVE_PHASE],
        );
        let painter = ui.painter();
        let y = mid - value * amp;
        let ring = Stroke::new(t, cx.theme.colors.text.to_color32());
        if mode == 0 {
            for cycle in 0..LFO_CYCLES as usize {
                let at = pos2(x_at((cycle as f32 + phase) / LFO_CYCLES), y);
                painter.circle_filled(at, t * 3.0, ink);
                painter.circle_stroke(at, t * 3.0, ring);
            }
        } else {
            painter.line_segment(
                [pos2(inner.left(), y), pos2(inner.right(), y)],
                Stroke::new(t * 0.75, ink.gamma_multiply(0.6)),
            );
            let at = pos2(x_at(if mode == 1 { phase } else { 0.5 }), y);
            painter.circle_filled(at, t * 3.0, ink);
            painter.circle_stroke(at, t * 3.0, ring);
        }
    }
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
    "LEVEL · OSC · MOD · FILTER · EG · LFO · MATRIX · VOICES · FX",
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
.h(Size::Flex(3.0))
.ink(Ink::Accent(Accent::Fx));

/// OSC `A` and `B`: the 29 waveshapes.
#[cfg(test)]
const SHAPES: &str = "Beast · Crunch · Double Spike · Digital · Dirt · Electro · Filth · Fox · Funky · Grubby · Keys · \
    Liquid · Ogre · Rhodes · Saw Mean · Saw Retro · Saw · Silk · Sine · Smooth Sub · Square Retro · Square Smooth · \
    Square · Strat · String · Throat · Triangle · Troll · Vocal · Sine to Saw · Saw to Square · Pulse Width · Vowels · Harmonic Sweep · FM Sweep · Hard Sync · Organ Bars · Bit Crush · Phase Scramble · Hollow Bell · Reed to Brass · Custom";

#[cfg(test)]
#[rustfmt::skip]
pub const REFERENCE: Reference = Reference {
    size: [16.96, 7.49],
    // A and B on Beast; PB RANGE, MW DEST and MW AMOUNT from the CSTM defaults (items 1 and 2, 0.5); FX slot 1; and
    // Honeydew's own FILTER EXTRA (16300) on Off and FM RANGE (32100) on x1 and the LFO's MODE, LOW and HIGH (65100–65300).
    settings: &[(16300, 0.0), (16400, 0.0), (21100, 0.0), (21200, 0.0), (32100, 0.0), (36100, 1.0), (36200, 2.0), (37000, 0.5), (65100, 0.0), (65200, 0.0), (65300, 1.0), (65400, 0.0), (65500, 0.0), (66000, 0.0), (66001, 0.0), (66002, 0.5), (66010, 0.0), (66011, 0.0), (66012, 0.5), (66020, 0.0), (66021, 0.0), (66022, 0.5), (66030, 0.0), (66031, 0.0), (66032, 0.5), (81100, 0.0)],
    cases: &[
        Case {
            name: "FILTER",
            tab: 3,
            rows: &[
                STRIP,
                row(12100, Widget::Display, "", [0.10, 1.35, 13.07, 3.45]).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Filter)),
                row(11000, Widget::Knob, "CUTOFF", [0.00, 4.90, 2.21, 2.59]).p(21).ink(Ink::Accent(Accent::Filter)),
                row(12000, Widget::Knob, "RES", [2.21, 4.90, 2.21, 2.59]).p(22).ink(Ink::Accent(Accent::Filter)),
                row(13000, Widget::Knob, "ATTACK", [4.42, 4.90, 2.21, 2.59]).p(25).ink(Ink::Accent(Accent::Filter)),
                row(14000, Widget::Knob, "DECAY", [6.63, 4.90, 2.21, 2.59]).p(26).ink(Ink::Accent(Accent::Filter)),
                row(15000, Widget::Knob, "AMOUNT", [8.84, 4.90, 2.22, 2.59]).p(27).ink(Ink::Accent(Accent::Filter)),
                row(16400, Widget::Knob, "DRIVE", [11.06, 4.90, 2.21, 2.59]).ink(Ink::Accent(Accent::Filter)),
                row(16100, Widget::TextToggle, "KBD", [13.27, 1.31, 3.69, 1.96]).p(24).ink(Ink::Accent(Accent::Filter)).margins(0.06, 0.0, 0.06, 0.0),
                row(16200, Widget::Spinner, "KIND", [13.27, 3.39, 3.69, 1.96]).p(23).ink(Ink::Accent(Accent::Filter)).margins(0.06, 0.0, 0.06, 0.0),
                row(16300, Widget::Spinner, "EXTRA", [13.27, 5.47, 3.69, 1.96]).items("Off · BP · Notch · Comb").ink(Ink::Accent(Accent::Filter)).margins(0.06, 0.0, 0.06, 0.0),
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
                row(12100, Widget::Display, "", [0.10, 1.35, 13.07, 3.75]).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Filter)),
                row(11000, Widget::Knob, "CUTOFF", [0.00, 5.20, 2.21, 2.80]).p(21).ink(Ink::Accent(Accent::Filter)),
                row(12000, Widget::Knob, "RES", [2.21, 5.20, 2.21, 2.80]).p(22).ink(Ink::Accent(Accent::Filter)),
                row(13000, Widget::Knob, "ATTACK", [4.42, 5.20, 2.21, 2.80]).p(25).ink(Ink::Accent(Accent::Filter)),
                row(14000, Widget::Knob, "DECAY", [6.63, 5.20, 2.21, 2.80]).p(26).ink(Ink::Accent(Accent::Filter)),
                row(15000, Widget::Knob, "AMOUNT", [8.84, 5.20, 2.22, 2.80]).p(27).ink(Ink::Accent(Accent::Filter)),
                row(16400, Widget::Knob, "DRIVE", [11.06, 5.20, 2.21, 2.80]).ink(Ink::Accent(Accent::Filter)),
                row(16100, Widget::TextToggle, "KBD", [13.27, 1.31, 3.69, 2.13]).p(24).ink(Ink::Accent(Accent::Filter)).margins(0.06, 0.0, 0.06, 0.0),
                row(16200, Widget::Spinner, "KIND", [13.27, 3.56, 3.69, 2.13]).p(23).ink(Ink::Accent(Accent::Filter)).margins(0.06, 0.0, 0.06, 0.0),
                row(16300, Widget::Spinner, "EXTRA", [13.27, 5.81, 3.69, 2.13]).items("Off · BP · Notch · Comb").ink(Ink::Accent(Accent::Filter)).margins(0.06, 0.0, 0.06, 0.0),
                row(1, Widget::Tabs, "LEVEL · OSC · MOD · FILTER · EG · LFO · MATRIX · VOICES · FX", [0.00, 8.00, 16.96, 1.25]).h(Size::U(1.25)),
                row(21100, Widget::Spinner, "A", [0.10, 9.35, 8.28, 2.85]).items(SHAPES).items_per_column(10).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Osc)),
                row(21200, Widget::Spinner, "B", [8.58, 9.35, 8.28, 2.85]).items(SHAPES).items_per_column(10).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Osc)),
                row(27000, Widget::Knob, "POS A", [0.00, 12.30, 8.48, 1.61]).p(75).ink(Ink::Accent(Accent::Osc)),
                row(28000, Widget::Knob, "POS B", [8.48, 12.30, 8.48, 1.61]).p(76).ink(Ink::Accent(Accent::Osc)),
                row(22000, Widget::Knob, "PITCH B", [0.00, 13.91, 3.39, 2.09]).p(11).ink(Ink::Accent(Accent::Osc)),
                row(23000, Widget::Knob, "FINE B", [3.39, 13.91, 3.39, 2.09]).p(12).ink(Ink::Accent(Accent::Osc)),
                row(24000, Widget::Knob, "PHASE B", [6.78, 13.91, 3.39, 2.09]).p(14).ink(Ink::Accent(Accent::Osc)),
                row(25000, Widget::Knob, "BALANCE", [10.17, 13.91, 3.39, 2.09]).p(13).ink(Ink::Accent(Accent::Osc)),
                row(26000, Widget::Knob, "NOISE", [13.56, 13.91, 3.40, 2.09]).p(20).ink(Ink::Accent(Accent::Osc)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "OSC",
            tab: 1,
            rows: &[
                STRIP,
                row(21100, Widget::Spinner, "A", [0.10, 1.35, 8.28, 2.62]).items(SHAPES).items_per_column(10).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Osc)),
                row(21200, Widget::Spinner, "B", [8.58, 1.35, 8.28, 2.62]).items(SHAPES).items_per_column(10).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Osc)),
                row(27000, Widget::Knob, "POS A", [0.00, 4.07, 8.48, 1.48]).p(75).ink(Ink::Accent(Accent::Osc)),
                row(28000, Widget::Knob, "POS B", [8.48, 4.07, 8.48, 1.48]).p(76).ink(Ink::Accent(Accent::Osc)),
                row(22000, Widget::Knob, "PITCH B", [0.00, 5.55, 3.39, 1.94]).p(11).ink(Ink::Accent(Accent::Osc)),
                row(23000, Widget::Knob, "FINE B", [3.39, 5.55, 3.39, 1.94]).p(12).ink(Ink::Accent(Accent::Osc)),
                row(24000, Widget::Knob, "PHASE B", [6.78, 5.55, 3.39, 1.94]).p(14).ink(Ink::Accent(Accent::Osc)),
                row(25000, Widget::Knob, "BALANCE", [10.17, 5.55, 3.39, 1.94]).p(13).ink(Ink::Accent(Accent::Osc)),
                row(26000, Widget::Knob, "NOISE", [13.56, 5.55, 3.40, 1.94]).p(20).ink(Ink::Accent(Accent::Osc)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "MOD",
            tab: 2,
            rows: &[
                STRIP,
                row(31100, Widget::Display, "", [0.10, 1.35, 13.07, 3.36]).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Mod)),
                row(31000, Widget::Knob, "MOD. AMNT", [0.00, 4.81, 2.65, 2.68]).p(15).ink(Ink::Accent(Accent::Mod)),
                row(33000, Widget::Knob, "ATTACK", [2.65, 4.81, 2.65, 2.68]).p(17).ink(Ink::Accent(Accent::Mod)),
                row(34000, Widget::Knob, "DECAY", [5.30, 4.81, 2.66, 2.68]).p(18).ink(Ink::Accent(Accent::Mod)),
                row(35000, Widget::Knob, "EG AMOUNT", [7.96, 4.81, 2.65, 2.68]).p(19).ink(Ink::Accent(Accent::Mod)),
                row(37000, Widget::Knob, "MW AMOUNT", [10.61, 4.81, 2.66, 2.68]).ink(Ink::Accent(Accent::Mod)),
                row(32000, Widget::Spinner, "KIND", [13.27, 1.31, 3.69, 1.44]).p(16).ink(Ink::Accent(Accent::Mod)).margins(0.06, 0.0, 0.06, 0.0),
                row(32100, Widget::Spinner, "FM RANGE", [13.27, 2.87, 3.69, 1.44]).items("x1 · x4 · x8 · x16").ink(Ink::Accent(Accent::Mod)).margins(0.06, 0.0, 0.06, 0.0),
                row(36100, Widget::Spinner, "PB RANGE", [13.27, 4.43, 3.69, 1.44]).items("1 smt · 2 smt · 7 smt · Oct").ink(Ink::Accent(Accent::Mod)).margins(0.06, 0.0, 0.06, 0.0),
                row(36200, Widget::Spinner, "MW DEST", [13.27, 5.99, 3.69, 1.44]).items("Filter · LFO · Osc. Mod.").ink(Ink::Accent(Accent::Mod)).margins(0.06, 0.0, 0.06, 0.0),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "LEVEL",
            tab: 0,
            rows: &[
                STRIP,
                row(41100, Widget::Display, "", [0.10, 1.35, 16.76, 3.51]).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Level)),
                row(41000, Widget::Knob, "LEVEL", [0.00, 4.96, 2.82, 2.53]).p(1).ink(Ink::Accent(Accent::Level)),
                row(42000, Widget::Knob, "PAN", [2.82, 4.96, 2.83, 2.53]).p(2).ink(Ink::Accent(Accent::Level)),
                row(43000, Widget::Knob, "ATK", [5.65, 4.96, 2.83, 2.53]).p(3).ink(Ink::Accent(Accent::Level)),
                row(44000, Widget::Knob, "DEC", [8.48, 4.96, 2.82, 2.53]).p(4).ink(Ink::Accent(Accent::Level)),
                row(45000, Widget::Knob, "SUS", [11.30, 4.96, 2.83, 2.53]).p(5).ink(Ink::Accent(Accent::Level)),
                row(46000, Widget::Knob, "REL", [14.13, 4.96, 2.82, 2.53]).p(6).ink(Ink::Accent(Accent::Level)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "EG",
            tab: 4,
            rows: &[
                STRIP,
                row(51100, Widget::Display, "", [0.10, 1.35, 13.07, 3.36]).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Envelope)),
                row(51000, Widget::Knob, "AMOUNT", [0.00, 4.81, 2.65, 2.68]).p(40).ink(Ink::Accent(Accent::Envelope)),
                row(52000, Widget::Knob, "ATTACK", [2.65, 4.81, 2.65, 2.68]).p(36).ink(Ink::Accent(Accent::Envelope)),
                row(53000, Widget::Knob, "DECAY", [5.30, 4.81, 2.66, 2.68]).p(37).ink(Ink::Accent(Accent::Envelope)),
                row(54000, Widget::Knob, "SUSTAIN", [7.96, 4.81, 2.65, 2.68]).p(38).ink(Ink::Accent(Accent::Envelope)),
                row(55000, Widget::Knob, "RELEASE", [10.61, 4.81, 2.66, 2.68]).p(39).ink(Ink::Accent(Accent::Envelope)),
                row(56100, Widget::TextToggle, "VEL", [13.27, 1.31, 3.69, 3.00]).p(74).ink(Ink::Accent(Accent::Envelope)).margins(0.06, 0.0, 0.06, 0.0),
                row(56200, Widget::Spinner, "DEST", [13.27, 4.43, 3.69, 3.00]).p(41).ink(Ink::Accent(Accent::Envelope)).margins(0.06, 0.0, 0.06, 0.0),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "LFO",
            tab: 5,
            rows: &[
                STRIP,
                row(61100, Widget::Display, "", [0.10, 1.35, 13.07, 3.64]).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Lfo)),
                row(61000, Widget::Knob, "AMOUNT", [0.00, 5.09, 8.84, 2.40]).p(34).ink(Ink::Accent(Accent::Lfo)),
                row(65500, Widget::TextToggle, "KEY", [8.84, 5.09, 4.43, 2.40]).w(Size::Flex(0.5)).ink(Ink::Accent(Accent::Lfo)),
                row(65100, Widget::Spinner, "MODE", [13.27, 1.31, 3.69, 1.12]).items("Shape · S&H · Note").margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
                row(62000, Widget::Spinner, "SHAPE", [13.27, 2.55, 3.69, 1.13]).p(33).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
                row(63000, Widget::Spinner, "DEST", [13.27, 3.80, 3.69, 1.13]).p(35).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
                row(64000, Widget::Spinner, "RATE", [13.27, 5.05, 3.69, 1.13]).p(32).items_per_column(5).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
                row(65400, Widget::Spinner, "SYNC", [13.27, 6.30, 3.69, 1.13]).items("Orig. · 1/32 · 1/16T · 1/16 · 1/8T · 1/16. · 1/8 · 1/4T · 1/8. · 1/4 · 1/2 · 1 Bar · 2 Bars · 4 Bars · 8 Bars").items_per_column(8).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "LFO random",
            tab: 5,
            settings: &[(65100, 1.0)],
            rows: &[
                STRIP,
                row(61100, Widget::Display, "", [0.10, 1.35, 13.07, 3.64]).margins(0.1, 0.1, 0.1, 0.1).ink(Ink::Accent(Accent::Lfo)),
                row(61000, Widget::Knob, "AMOUNT", [0.00, 5.09, 3.79, 2.40]).p(34).ink(Ink::Accent(Accent::Lfo)),
                row(65500, Widget::TextToggle, "KEY", [3.79, 5.09, 1.89, 2.40]).w(Size::Flex(0.5)).ink(Ink::Accent(Accent::Lfo)),
                row(65200, Widget::Knob, "LOW", [5.68, 5.09, 3.80, 2.40]).ink(Ink::Accent(Accent::Lfo)),
                row(65300, Widget::Knob, "HIGH", [9.48, 5.09, 3.79, 2.40]).ink(Ink::Accent(Accent::Lfo)),
                row(65100, Widget::Spinner, "MODE", [13.27, 1.31, 3.69, 1.12]).items("Shape · S&H · Note").margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
                row(62000, Widget::Spinner, "SHAPE", [13.27, 2.55, 3.69, 1.13]).p(33).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
                row(63000, Widget::Spinner, "DEST", [13.27, 3.80, 3.69, 1.13]).p(35).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
                row(64000, Widget::Spinner, "RATE", [13.27, 5.05, 3.69, 1.13]).p(32).items_per_column(5).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
                row(65400, Widget::Spinner, "SYNC", [13.27, 6.30, 3.69, 1.13]).items("Orig. · 1/32 · 1/16T · 1/16 · 1/8T · 1/16. · 1/8 · 1/4T · 1/8. · 1/4 · 1/2 · 1 Bar · 2 Bars · 4 Bars · 8 Bars").items_per_column(8).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Lfo)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "MATRIX",
            tab: 6,
            rows: &[
                STRIP,
                row(66500, Widget::Display, "", [0.00, 1.25, 16.96, 0.90]).h(Size::U(0.9)).ink(Ink::Accent(Accent::Lfo)),
                row(66000, Widget::Spinner, "FROM", [0.06, 2.21, 4.64, 1.27]).items("Off · LFO · EG").ink(Ink::Accent(Accent::Lfo)),
                row(66001, Widget::Spinner, "TO", [0.06, 3.48, 4.64, 1.28]).items("Off · Level · Pan · Osc B Pitch · Osc B Detune · Balance · Mod Amount · Noise · Cutoff · Resonance · Filter EG · EG Amount · LFO Amount · Choir Detune · Choir Stereo · FX 1 X · FX 1 Y · FX 2 X · Table Pos A · Table Pos B").items_per_column(6).ink(Ink::Accent(Accent::Lfo)),
                row(66002, Widget::Knob, "AMOUNT", [4.70, 2.21, 3.72, 2.55]).ink(Ink::Accent(Accent::Lfo)),
                row(66010, Widget::Spinner, "FROM", [8.54, 2.21, 4.64, 1.27]).items("Off · LFO · EG").ink(Ink::Accent(Accent::Lfo)),
                row(66011, Widget::Spinner, "TO", [8.54, 3.48, 4.64, 1.28]).items("Off · Level · Pan · Osc B Pitch · Osc B Detune · Balance · Mod Amount · Noise · Cutoff · Resonance · Filter EG · EG Amount · LFO Amount · Choir Detune · Choir Stereo · FX 1 X · FX 1 Y · FX 2 X · Table Pos A · Table Pos B").items_per_column(6).ink(Ink::Accent(Accent::Lfo)),
                row(66012, Widget::Knob, "AMOUNT", [13.18, 2.21, 3.72, 2.55]).ink(Ink::Accent(Accent::Lfo)),
                row(66020, Widget::Spinner, "FROM", [0.06, 4.88, 4.64, 1.27]).items("Off · LFO · EG").ink(Ink::Accent(Accent::Lfo)),
                row(66021, Widget::Spinner, "TO", [0.06, 6.15, 4.64, 1.28]).items("Off · Level · Pan · Osc B Pitch · Osc B Detune · Balance · Mod Amount · Noise · Cutoff · Resonance · Filter EG · EG Amount · LFO Amount · Choir Detune · Choir Stereo · FX 1 X · FX 1 Y · FX 2 X · Table Pos A · Table Pos B").items_per_column(6).ink(Ink::Accent(Accent::Lfo)),
                row(66022, Widget::Knob, "AMOUNT", [4.70, 4.88, 3.72, 2.55]).ink(Ink::Accent(Accent::Lfo)),
                row(66030, Widget::Spinner, "FROM", [8.54, 4.88, 4.64, 1.27]).items("Off · LFO · EG").ink(Ink::Accent(Accent::Lfo)),
                row(66031, Widget::Spinner, "TO", [8.54, 6.15, 4.64, 1.28]).items("Off · Level · Pan · Osc B Pitch · Osc B Detune · Balance · Mod Amount · Noise · Cutoff · Resonance · Filter EG · EG Amount · LFO Amount · Choir Detune · Choir Stereo · FX 1 X · FX 1 Y · FX 2 X · Table Pos A · Table Pos B").items_per_column(6).ink(Ink::Accent(Accent::Lfo)),
                row(66032, Widget::Knob, "AMOUNT", [13.18, 4.88, 3.72, 2.55]).ink(Ink::Accent(Accent::Lfo)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "VOICES",
            tab: 7,
            rows: &[
                STRIP,
                row(71000, Widget::Knob, "TRNSP", [0.00, 1.25, 3.31, 6.24]).p(28).ink(Ink::Accent(Accent::Level)),
                row(72000, Widget::Knob, "SLIDE", [3.31, 1.25, 3.32, 6.24]).p(29).ink(Ink::Accent(Accent::Level)),
                row(73000, Widget::Knob, "CHOIR DTN", [6.63, 1.25, 3.32, 6.24]).p(44).ink(Ink::Accent(Accent::Level)),
                row(74000, Widget::Knob, "CHOIR STR", [9.95, 1.25, 3.32, 6.24]).p(45).ink(Ink::Accent(Accent::Level)),
                row(75100, Widget::TextToggle, "MONO", [13.27, 1.31, 3.69, 1.44]).p(30).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Level)),
                row(75200, Widget::TextToggle, "AUTO", [13.27, 2.87, 3.69, 1.44]).p(31).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Level)),
                row(76100, Widget::Spinner, "VOICES", [13.27, 4.43, 3.69, 1.44]).p(42).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Level)),
                row(76200, Widget::Spinner, "SCALE", [13.27, 5.99, 3.69, 1.44]).p(43).margins(0.06, 0.0, 0.06, 0.0).ink(Ink::Accent(Accent::Level)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "FX slot 1",
            tab: 8,
            rows: &[
                STRIP,
                SLOTS,
                row(81200, Widget::Toggle, "", [0.00, 5.93, 3.39, 1.56]).p(47).ink(Ink::Accent(Accent::Fx)),
                row(82111, Widget::Spinner, "FX KIND", [3.39, 1.25, 3.39, 3.12]).p(46).items_per_column(5).ink(Ink::Accent(Accent::Fx)),
                row(82112, Widget::Knob, "AMNT", [6.78, 1.25, 3.39, 3.12]).p(50).ink(Ink::Accent(Accent::Fx)),
                row(82121, Widget::Spinner, "LFO RATE", [3.39, 4.37, 3.39, 3.12]).p(48).items_per_column(5).ink(Ink::Accent(Accent::Fx)),
                row(82122, Widget::Spinner, "LFO SHAPE", [6.78, 4.37, 3.39, 3.12]).p(49).ink(Ink::Accent(Accent::Fx)),
                row(82200, Widget::Knob, "X", [10.17, 1.25, 3.39, 6.24]).p(51).ink(Ink::Accent(Accent::Fx)),
                row(82300, Widget::Knob, "Y", [13.56, 1.25, 3.40, 6.24]).p(52).ink(Ink::Accent(Accent::Fx)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "FX slot 2",
            tab: 8,
            settings: &[(81100, 1.0)],
            rows: &[
                STRIP,
                SLOTS,
                row(81300, Widget::Toggle, "", [0.00, 5.93, 3.39, 1.56]).p(54).ink(Ink::Accent(Accent::Fx)),
                row(83111, Widget::Spinner, "FX KIND", [3.39, 1.25, 3.39, 3.12]).p(53).items_per_column(5).ink(Ink::Accent(Accent::Fx)),
                row(83112, Widget::Knob, "AMNT", [6.78, 1.25, 3.39, 3.12]).p(57).ink(Ink::Accent(Accent::Fx)),
                row(83121, Widget::Spinner, "LFO RATE", [3.39, 4.37, 3.39, 3.12]).p(55).items_per_column(5).ink(Ink::Accent(Accent::Fx)),
                row(83122, Widget::Spinner, "LFO SHAPE", [6.78, 4.37, 3.39, 3.12]).p(56).ink(Ink::Accent(Accent::Fx)),
                row(83200, Widget::Knob, "X", [10.17, 1.25, 3.39, 6.24]).p(58).ink(Ink::Accent(Accent::Fx)),
                row(83300, Widget::Knob, "Y", [13.56, 1.25, 3.40, 6.24]).p(59).ink(Ink::Accent(Accent::Fx)),
            ],
            ..Case::DEFAULT
        },
        Case {
            name: "FX slot 3",
            tab: 8,
            settings: &[(81100, 2.0)],
            rows: &[
                STRIP,
                SLOTS,
                row(81400, Widget::Toggle, "", [0.00, 5.93, 3.39, 1.56]).p(61).ink(Ink::Accent(Accent::Fx)),
                row(84111, Widget::Spinner, "FX KIND", [3.39, 1.25, 3.39, 3.12]).p(60).items_per_column(5).ink(Ink::Accent(Accent::Fx)),
                row(84112, Widget::Knob, "AMNT", [6.78, 1.25, 3.39, 3.12]).p(64).ink(Ink::Accent(Accent::Fx)),
                row(84121, Widget::Spinner, "LFO RATE", [3.39, 4.37, 3.39, 3.12]).p(62).items_per_column(5).ink(Ink::Accent(Accent::Fx)),
                row(84122, Widget::Spinner, "LFO SHAPE", [6.78, 4.37, 3.39, 3.12]).p(63).ink(Ink::Accent(Accent::Fx)),
                row(84200, Widget::Knob, "X", [10.17, 1.25, 3.39, 6.24]).p(65).ink(Ink::Accent(Accent::Fx)),
                row(84300, Widget::Knob, "Y", [13.56, 1.25, 3.40, 6.24]).p(66).ink(Ink::Accent(Accent::Fx)),
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
    fn a_slow_b_acts_around_the_middle_of_the_picture_not_from_one_side() {
        let (a, b) = cycles(SINE, SINE);
        // B two octaves down, at the peak of its sine in the middle of A's cycle: ring modulation is then odd about the
        // middle (A is, and B's factor is even about its peak), whatever B's pitch.
        let mut module = module(0.1, 0.5);
        module.params[11] = 0.0;
        module.params[14] = 0.25;
        let n = 400;
        let curve = curve_a(&a, &b, &module, &PanelState::default(), n);
        for i in 0..=n / 2 {
            let (left, right) = (curve[i], curve[n - i]);
            assert!(
                (left + right).abs() < 0.02,
                "{i}: {left} and {right} are not mirror images"
            );
        }
    }

    #[test]
    fn a_slow_b_shows_the_cycles_of_a_changing_not_one_unchanged_cycle() {
        let (a, b) = cycles(SAW, SINE);
        let mut state = PanelState::default();
        state.settings.insert(32100, 2.0); // FM RANGE x8
        let mut module = module(0.4, 0.2); // FM, 20 %
        module.params[11] = 0.0; // PITCH B -24: two octaves down
        let n = 800;
        let curve = curve_a(&a, &b, &module, &state, n);
        // The window opens to four cycles of A, 200 points each: the first and the last differ, as they sound.
        let (first, last) = (&curve[..200], &curve[600..800]);
        let difference: f32 = first.iter().zip(last).map(|(x, y)| (x - y).abs()).sum();
        assert!(difference > 5.0, "{difference}");
        // A fast B keeps the window at one cycle.
        module.params[11] = 0.75;
        let fast = curve_a(&a, &b, &module, &state, 200);
        assert_eq!(fast.len(), 201);
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
