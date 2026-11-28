//! GMSynth (instrument ID 2), term by term. Two additive-wavetable
//! oscillators with choir copies and osc mod, noise, an 8-stage one-pole filter, four one-pole envelopes, one global
//! LFO, mod wheel and pitch bend, master level and pan. vₙ below is param n. Rates are per sample unless marked
//! "block" (32 samples); the block grid runs on across `process` calls, so any block split renders alike.
//!
//! The summed voice bus runs FX 1 → FX 2 → FX 3 (p46–p66), each a MultiFX processor bank with DIST and RING
//! ([`crate::multifx`]), then the master level and pan.
//!
//! Tables are built on the app thread and shared:
//! - **Shapes.** A process-wide cache holds one `Arc<Wavetable>` per menu slot, built on first use by [`GmSynth::new`]
//!   (the default shape) and [`prepare_setting`]. [`Instrument::set_setting`] runs on the audio thread and only takes
//!   an `Arc` clone of a built slot: an atomic increment, no allocation or lock. The cache keeps its own reference, so
//!   dropping a voice's old shape there never frees it. A slot not yet built reads as a missing shape (silent).
//! - **The filter table** is built once per process, at the first construction.

use std::f32::consts::TAU;
use std::f64::consts::{PI, TAU as TAU_64};
use std::sync::{Arc, OnceLock};

use crate::ProcessContext;
use crate::gm_waveshapes::{
    SLOTS, Spectrum, TABLE_LEN, Wavetable, read, spectrum, table_index_for,
};
use crate::instrument::Instrument;
use crate::multifx::{Bank, Settings};
use crate::spectrum::SpectrumProbe;
use crate::voice::{GMSYNTH_RATE_BEATS, tempo_step};
use crate::wavetable::{FramePair, FrameTable};
use crate::wavetable_factory;

/// Params 0–76 (75 and 76 are Honeydew's own: the wavetable positions of OSC A and B).
pub const PARAMS: usize = 77;
/// The panel's live values ([`Instrument::analysis`]): the LFO's value now (−1…1), its phase now (0…1), the filter's
/// cutoff now (0…1), then the output's [`crate::spectrum::BANDS`] band levels (0…1, 20 Hz to 20 kHz).
pub const LIVE_LFO: usize = 0;
pub const LIVE_PHASE: usize = 1;
/// The filter's cutoff now (0…1), as the sounding voices have it with their envelopes, LFO and key tracking applied: their
/// mean, or the CUTOFF knob's value while nothing sounds.
pub const LIVE_CUTOFF: usize = 2;
pub const LIVE_BANDS: usize = 3;
/// The length of the live values.
pub const LIVE_LEN: usize = LIVE_BANDS + crate::spectrum::BANDS;
/// Voices.
pub const VOICES: usize = 4;
/// Choir copies at most: Solo … Quartet.
const COPIES: usize = 4;
/// The control block: "block" laws update every 32 samples.
const BLOCK: usize = 32;

/// Param defaults. A menu's default is its exact step k/(n − 1): the table's
/// 0.1111 is 1/9 rounded for display.
#[rustfmt::skip]
pub const DEFAULTS: [f32; PARAMS] = [
    1.0, 0.7273, 0.5, 0.0, 1.0, 1.0, 0.2, 0.5, 0.5, 0.5, 0.5, // 0–10
    0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.0, // 11–20
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.5, 0.0, 0.0, // 21–30
    0.0, 1.0, 0.0, 0.5, 0.0, 0.0, 1.0, 1.0, 1.0, 0.5, // 31–40
    0.0, 0.0, 0.0, 0.0, 0.0, 1.0 / 9.0, 0.0, 0.0, 0.0, 0.5, // 41–50
    0.5, 0.5, 5.0 / 9.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 3.0 / 9.0, // 51–60
    0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0, // 61–70
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 71–76
];
/// Enable is on at v ≥ 0.5; MONO (p30), AUTO (p31) and VEL (p74) at any nonzero v.
const ENABLE: usize = 0;
const SINGLE_VOICE: usize = 30;
/// FX slot n's params start at 46 + 7n: Kind, Enable, Sync, LFO Shape, LFO Amount, X, Y.
const FX_PARAMS: usize = 46;
const FX_SLOTS: usize = 3;

/// `CSTM` settings under the keys the panel uses (`src/panels/cstm.rs` `GMSYNTH`).
/// A menu's setting is its item index: OSC A / B shape 0–28, PB RANGE item, MW DEST item; MW AMOUNT is v.
pub const WAVE_A: u32 = 21100;
pub const WAVE_B: u32 = 21200;
pub const PB_RANGE: u32 = 36100;
pub const MW_AMOUNT: u32 = 37000;
pub const MW_DEST: u32 = 36200;
/// Honeydew's extra filter types (not in FL Studio Mobile): item 0 `Off` keeps the FILTER `KIND` param (v23), items
/// 1–3 `BP`, `Notch` and `Comb` replace it. Kept in the `CSTM` blob after the original 20 bytes.
pub const FILTER_EXTRA: u32 = 16300;
/// Honeydew's filter DRIVE (0…1, off at 0): the voice is pushed into the filter's own soft clip ([`CLIP`]) before it, as
/// Serum's Drive does, for grit and body without a louder output. Kept in the `CSTM` blob after the modulation routes.
pub const FILTER_DRIVE: u32 = 16400;
/// The input gain at full DRIVE (1 at none): the voice, which is quiet (half a unit at most), is boosted until it
/// runs into [`drive_clip`], so the knob is heard from its first quarter on. The output gives back
/// `1 / (1 + 1.5·drive)`, so the sound gets denser rather than much louder.
const DRIVE_GAIN: f32 = 30.0;

/// A smooth, symmetric soft clip (a rational tanh, exact in its slope at 0, flat beyond ±3).
fn drive_clip(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
}
/// Honeydew's FM depth range (not in FL Studio Mobile): item 0 `x1` is the original depth, the others multiply the
/// phase offset FM adds (A is read at φ_A + (B + 1)·m·scale cycles), so a low amount already modulates hard. Kept in
/// the `CSTM` blob after [`FILTER_EXTRA`].
pub const FM_RANGE: u32 = 32100;
/// The FM scale of each [`FM_RANGE`] item.
pub const FM_SCALES: [f32; 4] = [1.0, 4.0, 8.0, 16.0];
/// Honeydew's LFO modes (not in FL Studio Mobile), kept in the `CSTM` blob after [`FM_RANGE`]: item 0 `Shape` is the
/// original LFO (the SHAPE menu's), 1 `S&H` takes a new random value at the start of each cycle and holds it, 2 `Note` takes
/// a new random value at each note and holds it. The values lie between [`LFO_LOW`] and [`LFO_HIGH`] (0…1 each, over the
/// LFO's −1…1).
pub const LFO_MODE: u32 = 65100;
pub const LFO_LOW: u32 = 65200;
pub const LFO_HIGH: u32 = 65300;
/// Honeydew's LFO sync menu: a note value for the LFO's cycle, in beats (item 0, `Orig.`, keeps the RATE menu's, whose steps
/// are 1/4 to 8 bars with no triplets and nothing faster than a sixteenth). Kept in the `CSTM` blob after [`LFO_HIGH`].
pub const LFO_SYNC: u32 = 65400;
/// The cycle of each [`LFO_SYNC`] item after `Orig.`: 1/32, 1/16T, 1/16, 1/8T, 1/16., 1/8, 1/4T, 1/8., 1/4, 1/2, 1 bar, 2, 4
/// and 8 bars, in beats of a quarter note.
pub const LFO_SYNC_BEATS: [f32; 14] = [
    0.125,
    1.0 / 6.0,
    0.25,
    1.0 / 3.0,
    0.375,
    0.5,
    2.0 / 3.0,
    0.75,
    1.0,
    2.0,
    4.0,
    8.0,
    16.0,
    32.0,
];
/// Honeydew's LFO key sync: with it on (item 1), each note restarts the LFO's cycle, so it does not run on between
/// notes. Kept in the `CSTM` blob after [`LFO_SYNC`].
pub const LFO_KEYSYNC: u32 = 65500;

/// Honeydew's free modulation routes (the "mod matrix"): [`MOD_SLOTS`] slots, each sending the LFO or the EG to any
/// parameter of [`MOD_TARGETS`] with a bipolar amount. A slot's three settings are `MOD_BASE + 10·slot + field`
/// ([`MOD_SOURCE`], [`MOD_TARGET`], [`MOD_AMOUNT`]), kept in the `CSTM` blob after [`LFO_KEYSYNC`], so a song that
/// uses no route keeps the original bytes. The older single destinations (`LFO Dest`, `EG Dest`) stay as they are.
pub const MOD_BASE: u32 = 66000;
pub const MOD_SLOTS: usize = 4;
pub const MOD_SOURCE: u32 = 0;
pub const MOD_TARGET: u32 = 1;
pub const MOD_AMOUNT: u32 = 2;
/// A route's source: 0 off, 1 the LFO (−1…1), 2 the EG (0…1, the newest sounding voice's).
pub const MOD_SOURCES: [&str; 3] = ["Off", "LFO", "EG"];
/// The parameters a route may drive: only smooth knobs, never menus or switches (a modulated menu would flicker
/// between its items). Level, Pan, Osc B pitch/detune/mix, Osc Mod, Noise, Cutoff, Resonance, Filter contour, EG
/// contour, LFO amount, Choir detune/stereo, and the three FX slots' X and Y.
pub const MOD_TARGETS: [usize; 19] = [
    1, 2, 11, 12, 13, 15, 20, 21, 22, 27, 40, 34, 44, 45, 51, 52, 58, POS_A, POS_B,
];
/// A route's target menu: `Off` first (item 0, no route), then [`MOD_TARGETS`] in order.
pub const MOD_TARGET_NAMES: [&str; 20] = [
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

/// One modulation route as the engine holds it.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Route {
    source: u8,
    /// Index into [`MOD_TARGETS`]; `None` is the menu's `Off`.
    target: Option<usize>,
    /// −1…1.
    amount: f32,
}
const LFO_SAMPLE_HOLD: u8 = 1;
const LFO_PER_NOTE: u8 = 2;
const EXTRA_BP: u8 = 1;
const EXTRA_NOTCH: u8 = 2;
const EXTRA_COMB: u8 = 3;
/// PB RANGE items `1 smt`, `2 smt`, `7 smt`, `Oct`; default item 1.
const PB_RANGES: [f32; 4] = [1.0, 2.0, 7.0, 12.0];
/// MW DEST items; default Osc. Mod.
const MW_FILTER: usize = 0;
const MW_LFO: usize = 1;
const MW_OSC_MOD: usize = 2;
/// Shape at creation: Beast.
const DEFAULT_SHAPE: usize = 0;

/// Channel params as [`Instrument::set_controller`] indexes them: Pitch Bend and MIDI CC #1,
/// the mod wheel.
pub const PITCH_BEND: usize = 4;
pub const MOD_WHEEL: usize = 8;

// Osc mod kinds, int(3·v16).
const RING: u8 = 0;
const FM: u8 = 1;
const SYNC: u8 = 2;
const FILTER: u8 = 3;
/// Ring, FM and Filter mod are inactive at m ≤ 0.001.
const MOD_THRESHOLD: f32 = 0.001;

/// One-pole smoothing per sample: v15 + MW, voice gain, master level and pan (authored at 44.1 kHz).
const SMOOTHING: f32 = 0.01;
/// The cutoff base S follows its target at 0.001 per sample.
const CUTOFF_SMOOTHING: f32 = 0.001;

/// One-pole smoothing per sample of the wavetable positions (about 4 ms): a position that jumps (a knob dragged fast, an LFO
/// stepped once per control block) glides to its new value instead of switching the waveform in one sample, which clicks.
const POSITION_SMOOTHING: f32 = 0.005;
/// KBD tracking per key around key 64: 1.0008 oct/oct.
const KBD_PER_KEY: f32 = 0.008_996_4;
const KBD_CENTRE: f32 = 64.0;
/// Attack aims here and hands over to decay once above 1.
const ATTACK_TARGET: f32 = 5.0;
/// The envelope rate numerator: k = 8.82/(sr·…).
const RATE_SCALE: f32 = 8.82;
/// Envelopes below this are 0, so a long decay to 0 never runs on subnormals (ours, far below hearing).
const FLUSH: f32 = 1e-20;
/// A voice whose amp env aims at 0 goes idle below this level.
const FREE_LEVEL: f32 = 1e-4;
/// The master level is 0 below −60 dB.
const SILENT_DB: f32 = -60.0;

/// Nominal cutoff f = 32·2^(9.27c) Hz.
const CUTOFF_HZ: f32 = 32.0;
const CUTOFF_OCTAVES: f32 = 9.27;
/// The rate the filter table is authored for; elsewhere its frequencies scale with sr/44100.
const TABLE_RATE: f64 = 44_100.0;
/// The table entry for f peaks at f_r ≈ 1.0159·f + 0.5 Hz (a fit of the shipped table, residual < 1 Hz).
const PEAK_SCALE: f32 = 1.0159;
const PEAK_OFFSET_HZ: f32 = 0.5;
/// The regenerated table's axis: entry j peaks at f_r = 11.68 + 0.304433·j Hz (a fit, not an exact constant), read
/// with linear interpolation between entries.
const AXIS_START_HZ: f32 = 11.68;
const AXIS_STEP_HZ: f32 = 0.304_433;
/// Entries (65536 puts the last at 19.96 kHz, where b = 9.86, the top; entry 0 gives the low 1.888).
const FILTER_TABLE_LEN: usize = 65_536;
/// One-pole stages per channel.
const STAGES: usize = 8;
/// Soft clip x/(1 + 0.05|x|) at the filter input.
const CLIP: f32 = 0.05;

/// The xorshift seed of the dither and phase generator, and the base seed of each voice's noise (our choice).
const SEED: u32 = 0x2545_F491;

/// The 29 shapes' tables, each built once per process and shared by every instance ("Shared by all instances").
static SHAPES: [OnceLock<Arc<Wavetable>>; SLOTS] = [const { OnceLock::new() }; SLOTS];
/// The built-in wavetables, built on first use (app thread) and shared by every instance.
static FACTORY: [OnceLock<Arc<FrameTable>>; wavetable_factory::NAMES.len()] =
    [const { OnceLock::new() }; wavetable_factory::NAMES.len()];

/// The OSC A / B menu's items: the 29 classic shapes (0–28), then the built-in wavetables ([`TABLE_BASE`]…).
pub const SOURCES: usize = SLOTS + wavetable_factory::NAMES.len();
/// The menu item of the first built-in wavetable.
pub const TABLE_BASE: usize = SLOTS;
/// The OSC A / B position params (Honeydew's own): where in its wavetable an oscillator reads, 0…1. A classic shape
/// ignores it.
pub const POS_A: usize = 75;
pub const POS_B: usize = 76;

/// What an oscillator reads: one of the classic shapes (nearest point plus dither, as the original), or a wavetable
/// that morphs with the position param and reads with interpolation.
#[derive(Clone)]
enum Source {
    Shape(Arc<Wavetable>),
    Frames(Arc<FrameTable>),
}

/// One oscillator's table for a control block's worth of samples.
enum Reader<'a> {
    Shape(&'a [f32; TABLE_LEN]),
    Frames(FramePair<'a>),
}

impl Reader<'_> {
    #[inline]
    fn read(&self, phase: u32, dither: u32) -> f32 {
        match self {
            Self::Shape(table) => read(table, phase, dither),
            Self::Frames(pair) => pair.read(phase),
        }
    }
}

impl Source {
    /// The reader for band-limit `level` (a shape's table index or a wavetable's version) and `position`.
    fn reader(&self, level: usize, position: f32) -> Reader<'_> {
        match self {
            Self::Shape(wave) => Reader::Shape(wave.table(level)),
            Self::Frames(table) => Reader::Frames(table.pair(level, position)),
        }
    }
}
static FILTER_TABLE: OnceLock<FilterTable> = OnceLock::new();

/// App thread: builds what a `CSTM` setting needs before [`Instrument::set_setting`] receives it on the audio thread
/// (the OSC A / B shape). The engine calls it at build; a live setting edit calls it before sending.
pub fn prepare_setting(key: u32, value: f32) {
    if matches!(key, WAVE_A | WAVE_B) {
        let _ = source(value as usize);
    }
}

/// The user's original waveshapes, by slot, set at start ([`install_shape`]).
static ORIGINAL: [OnceLock<Spectrum>; SLOTS] = [const { OnceLock::new() }; SLOTS];

/// App thread, at start, before any GMSynth is built: menu slot `slot` plays `spectrum` (the user's own content) in
/// place of its designed shape. Returns whether it took it: once per slot and process, and never
/// after the slot's tables were built.
pub fn install_shape(slot: usize, spectrum: Spectrum) -> bool {
    let built = SHAPES.get(slot).is_none_or(|cell| cell.get().is_some());
    !built && ORIGINAL[slot].set(spectrum).is_ok()
}

/// App thread: menu slot `slot`'s tables, built on first use (2 MB, a few ms); `None` past the menu.
fn shape(slot: usize) -> Option<Arc<Wavetable>> {
    let cell = SHAPES.get(slot)?;
    let built = cell.get_or_init(|| {
        let original = ORIGINAL[slot].get().cloned();
        Arc::new(Wavetable::build(
            &original.or_else(|| spectrum(slot)).expect("a menu slot"),
        ))
    });
    Some(Arc::clone(built))
}

/// App thread: one cycle of menu slot `slot` with its lowest `harmonics` partials (1 to 512), 1024 samples, for a
/// display; `None` past the menu. Builds the slot's tables on first use, as [`prepare_setting`] does.
pub fn preview_cycle(slot: usize, harmonics: usize) -> Option<Vec<f32>> {
    let wave = shape(slot)?;
    let table = wave.table(harmonics.clamp(1, crate::gm_waveshapes::TABLES) - 1);
    Some(table.to_vec())
}

/// Any thread: `slot`'s tables if they are built. Never builds, allocates or blocks.
fn cached(slot: usize) -> Option<Arc<Wavetable>> {
    SHAPES.get(slot)?.get().cloned()
}

/// App thread: menu item `item` as an oscillator source, building its tables on first use; `None` past the menu.
fn source(item: usize) -> Option<Source> {
    if item < SLOTS {
        return shape(item).map(Source::Shape);
    }
    let cell = FACTORY.get(item - TABLE_BASE)?;
    let built = cell.get_or_init(|| {
        let cycles = wavetable_factory::cycles(item - TABLE_BASE).expect("a built-in table");
        Arc::new(FrameTable::from_cycles(&cycles).expect("a built-in table builds"))
    });
    Some(Source::Frames(Arc::clone(built)))
}

/// Any thread: menu item `item` as a source if it is built. Never builds, allocates or blocks.
fn cached_source(item: usize) -> Option<Source> {
    if item < SLOTS {
        return cached(item).map(Source::Shape);
    }
    FACTORY
        .get(item - TABLE_BASE)?
        .get()
        .cloned()
        .map(Source::Frames)
}

/// App thread: how many frames menu item `item` has when it is a wavetable (a classic shape has none), for the pictures.
pub fn table_frames(item: usize) -> Option<usize> {
    match source(item)? {
        Source::Frames(table) => Some(table.frames()),
        Source::Shape(_) => None,
    }
}

/// App thread: one cycle of menu item `item` as the pictures draw it, 1024 values: a classic shape with its lowest
/// `harmonics` partials ([`preview_cycle`]), or a wavetable at `position` (0…1, the frames blended as the oscillator
/// does). `None` past the menu.
pub fn source_cycle(item: usize, harmonics: usize, position: f32) -> Option<Vec<f32>> {
    match source(item)? {
        Source::Shape(_) => preview_cycle(item, harmonics),
        Source::Frames(table) => {
            let pair = table.pair(0, position);
            Some(
                (0..TABLE_LEN)
                    .map(|i| pair.read((i as u64 * (1 << 32) / TABLE_LEN as u64) as u32))
                    .collect(),
            )
        }
    }
}

/// App thread: wavetable `item`'s frame `frame` as `points` values over one cycle, for the pictures.
pub fn table_outline(item: usize, frame: usize, points: usize) -> Option<Vec<f32>> {
    match source(item)? {
        Source::Frames(table) => Some(table.outline(frame.min(table.frames() - 1), points)),
        Source::Shape(_) => None,
    }
}

/// The stage coefficient a and the self-oscillation gain b by f_r on the table axis.
struct FilterTable(Box<[[f32; 2]]>);

impl FilterTable {
    /// Entry j: a makes the 8-stage loop, with its 1-sample feedback delay, reach phase −π at ω = 2π·f_r/44100,
    /// i.e. 8·arg H − ω = −π for H = a/(1 − (1 − a)e^{−jω}); b = |H|^−8. With r = 1 − a and
    /// θ = (π − ω)/8, arg H = −atan2(r·sin ω, 1 − r·cos ω) = −θ solves to r = sin θ / sin(ω + θ).
    fn build() -> Self {
        Self(
            (0..FILTER_TABLE_LEN)
                .map(|j| {
                    let f_r = f64::from(AXIS_START_HZ) + f64::from(AXIS_STEP_HZ) * j as f64;
                    let w = TAU_64 * f_r / TABLE_RATE;
                    let theta = (PI - w) / 8.0;
                    let r = theta.sin() / (w + theta).sin();
                    let h = (1.0 - r) / (1.0 - 2.0 * r * w.cos() + r * r).sqrt();
                    [(1.0 - r) as f32, h.powi(-8) as f32]
                })
                .collect(),
        )
    }

    /// (a, b) for normalized cutoff c: nominal f = 32·2^(9.27c), its f_r on the axis, interpolated.
    fn at(&self, c: f32) -> (f32, f32) {
        let f = CUTOFF_HZ * (CUTOFF_OCTAVES * c).exp2();
        let x = ((PEAK_SCALE * f + PEAK_OFFSET_HZ - AXIS_START_HZ) / AXIS_STEP_HZ)
            .clamp(0.0, (FILTER_TABLE_LEN - 1) as f32);
        let j = (x as usize).min(FILTER_TABLE_LEN - 2);
        let t = x - j as f32;
        let ([a0, b0], [a1, b1]) = (self.0[j], self.0[j + 1]);
        (a0 + t * (a1 - a0), b0 + t * (b1 - b0))
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
enum Stage {
    #[default]
    Idle,
    Attack,
    Decay,
    Release,
}

/// Per-sample envelope coefficients, refreshed each block.
#[derive(Clone, Copy, Default)]
struct Rates {
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
}

/// e += (target − e)·k per sample. Attack aims at 5.0 and ends when e > 1 (e := 1); decay aims at the sustain
/// level, release at 0.
#[derive(Clone, Copy, Default)]
struct Envelope {
    e: f32,
    stage: Stage,
}

impl Envelope {
    fn step(&mut self, rates: &Rates) -> f32 {
        match self.stage {
            Stage::Attack => {
                self.e += (ATTACK_TARGET - self.e) * rates.attack;
                if self.e > 1.0 {
                    self.e = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => self.e += (rates.sustain - self.e) * rates.decay,
            Stage::Release => self.e -= self.e * rates.release,
            Stage::Idle => {}
        }
        if self.e < FLUSH {
            self.e = 0.0;
        }
        self.e
    }

    /// Restarts the attack from 0 (filter env, mod env, EG).
    fn restart(&mut self) {
        *self = Self {
            e: 0.0,
            stage: Stage::Attack,
        };
    }
}

/// Attack k = 1 at v = 0, else 8.82/(sr·100v⁴): 0 → 1 in ≈ 2.53·v⁴ s.
fn attack_rate(v: f32, sample_rate: f32) -> f32 {
    if v <= 0.0 {
        1.0
    } else {
        RATE_SCALE / (sample_rate * 100.0 * v.powi(4))
    }
}

/// Decay k = 8.82/(sr·(200v⁷ + 0.001)): τ = 22.7·v⁷ s + 0.11 ms.
fn decay_rate(v: f32, sample_rate: f32) -> f32 {
    RATE_SCALE / (sample_rate * (200.0 * v.powi(7) + 0.001))
}

/// Everything the block laws give, refreshed every 32 samples.
/// Which kind of table an oscillator reads, which decides how a voice picks its band-limit level.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum SourceKind {
    #[default]
    Shape,
    Frames,
}

impl SourceKind {
    /// The band-limit level for a voice at `freq`.
    fn level(self, freq: f32, sample_rate: f32) -> usize {
        match self {
            Self::Shape => table_index_for(freq, sample_rate),
            Self::Frames => FrameTable::level_for(freq, sample_rate),
        }
    }

    fn of(source: &Option<Source>) -> Self {
        match source {
            Some(Source::Frames(_)) => Self::Frames,
            _ => Self::Shape,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Control {
    /// What OSC A and B read ([`SourceKind`]) and their wavetable positions ([`POS_A`], [`POS_B`]).
    source_a: SourceKind,
    source_b: SourceKind,
    pos_a: f32,
    pos_b: f32,
    amp: Rates,
    filter_env: Rates,
    mod_env: Rates,
    eg: Rates,
    /// Choir N, each copy's frequency ratio and its [L, R] gains.
    copies: usize,
    ratio: [f64; COPIES],
    copy_pan: [[f32; 2]; COPIES],
    gain_a: f32,
    gain_b: f32,
    noise: f32,
    /// f_B/f_A.
    ratio_b: f32,
    /// TRNSP + R·w, in semitones.
    offset: f32,
    slide: f32,
    /// VEL: the Filter and Osc. Mod. EG depths × velocity.
    velocity_depth: bool,
    eg_filter: f32,
    eg_mod: f32,
    eg_pitch: f32,
    lfo_shape: u8,
    lfo_filter: f32,
    lfo_mod: f32,
    /// LFO → FREQ (st) and LFO → MOD at the block start, for the block laws.
    lfo_pitch: f32,
    lfo_mod_now: f32,
    mod_kind: u8,
    /// 2v19 − 1.
    mod_contour: f32,
    /// v15 + MW, the smoother's target; clamp(v15 + MW, 0, 1) for Sync's m′.
    mod_target: f32,
    sync_base: f32,
    /// (2v27 − 1)³.
    filter_contour: f32,
    cutoff: f32,
    kbd: f32,
    mw_filter: f32,
    resonance: f32,
    filter_kind: u8,
    /// [`FILTER_EXTRA`]: 0 uses `filter_kind`.
    filter_extra: u8,
    /// `CSTM` DRIVE, 0…1 ([`FILTER_DRIVE`]).
    drive: f32,
    /// The FM phase offset's scale ([`FM_SCALES`]).
    fm_scale: f32,
    sample_rate: f32,
    /// Master targets: linear level and pan P.
    level: f32,
    pan: f32,
}

#[derive(Clone, Copy, Default)]
struct Voice {
    key: u8,
    velocity: f32,
    /// The note pan p, and the voice pan [L, R] of clamp(p + slide pan, 0, 1).
    note_pan: f32,
    pan: [f32; 2],
    /// Slide offsets: semitones, pan, velocity; a launched voice starts at 0.
    slide: [f32; 3],
    /// Start serial; the smallest is the oldest.
    age: u64,
    /// The glide pitch p, in keys.
    p: f32,
    amp: Envelope,
    filter_env: Envelope,
    mod_env: Envelope,
    eg: Envelope,
    /// The smoothed velocity gain.
    gain: f32,
    /// Cutoff base S, its block target, and whether S snaps to it (note start).
    s: f32,
    s_target: f32,
    snap: bool,
    phase_a: [u32; COPIES],
    phase_b: [u32; COPIES],
    step_a: [u32; COPIES],
    step_b: [u32; COPIES],
    table_a: usize,
    table_b: usize,
    stages: [[f32; STAGES]; 2],
    /// The `Comb` filter type's delay lines.
    comb: CombLines,
    /// The cutoff the filter had at the last sample ([`LIVE_CUTOFF`]).
    last_cutoff: f32,
    /// The wavetable positions of OSC A and B as the voice reads them now, gliding to [`Control`]'s ([`POSITION_SMOOTHING`]).
    position: [f32; 2],
    noise: u32,
}

impl Voice {
    fn sounding(&self) -> bool {
        self.amp.stage != Stage::Idle
    }

    /// Sounding and not yet released.
    fn held(&self) -> bool {
        matches!(self.amp.stage, Stage::Attack | Stage::Decay)
    }

    /// Note-off: the amp env and EG release; the filter and mod envs ignore it.
    fn release(&mut self) {
        if self.sounding() {
            self.amp.stage = Stage::Release;
            self.eg.stage = Stage::Release;
        }
    }

    /// Glide, once per block: p += (target − p)·n/(0.1·sr·v29 + 1), snapping when it reaches the target.
    fn glide(&mut self, slide: f32, sample_rate: f32) {
        let gap = f32::from(self.key) - self.p;
        let step = gap * BLOCK as f32 / (0.1 * sample_rate * slide + 1.0);
        self.p = if step.abs() >= gap.abs() {
            f32::from(self.key)
        } else {
            self.p + step
        };
    }

    /// The block laws: pitch, both tables, the phase steps and the cutoff target.
    fn control(&mut self, c: &Control, sample_rate: f32) {
        // EG → Pitch and the EG term in Sync's m′ ignore VEL.
        let eg = self.eg.e;
        // Pitch = p + TRNSP + R·w + slide + [EG→Pitch] + [LFO→FREQ]; f_A = 220·2^((pitch − 57)/12).
        let pitch = self.p + c.offset + self.slide[0] + c.eg_pitch * eg + c.lfo_pitch;
        let f_a = 220.0 * ((pitch - 57.0) / 12.0).exp2();
        let f_b = f_a * c.ratio_b;
        self.table_a = c.source_a.level(f_a, sample_rate);
        self.table_b = c.source_b.level(f_b, sample_rate);
        // Sync: A runs at f_A·2^m′, m′ = clamp(v15 + MW, 0, 1) + (2v19 − 1)·E_mod + mods, unclamped.
        let sync = if c.mod_kind == SYNC {
            (c.sync_base + c.mod_contour * self.mod_env.e + c.eg_mod * eg + c.lfo_mod_now).exp2()
        } else {
            1.0
        };
        for (k, &ratio) in c.ratio.iter().enumerate() {
            self.step_a[k] = phase_step(f_a * sync, ratio, sample_rate);
            self.step_b[k] = phase_step(f_b, ratio, sample_rate);
        }
        self.s_target =
            (c.cutoff + c.kbd * (pitch - KBD_CENTRE) * KBD_PER_KEY + c.mw_filter).clamp(0.0, 1.0);
        if self.snap {
            self.s = self.s_target;
            // A new note starts on the positions as they are, not gliding from where the last note left them.
            self.position = [c.pos_a, c.pos_b];
            self.snap = false;
        }
    }

    /// Adds this voice into `bus` for one stretch of a block: `dither`, `lfo` and `amount` (the smoothed v15 + MW)
    /// hold one value per sample.
    #[allow(clippy::too_many_arguments)]
    fn render(
        &mut self,
        c: &Control,
        (wave_a, wave_b): (&Source, &Source),
        filter: &FilterTable,
        dither: &[u32],
        lfo: &[f32],
        amount: &[f32],
        bus: &mut [[f32; BLOCK]; 2],
    ) {
        // VEL scales the EG depth for the Filter and Osc. Mod. destinations only.
        let velocity = (self.velocity + self.slide[2]).clamp(0.0, 1.0);
        let depth = if c.velocity_depth { velocity } else { 1.0 };
        let sync = c.mod_kind == SYNC;
        for (i, &dither) in dither.iter().enumerate() {
            let amp = self.amp.step(&c.amp);
            let e_filter = self.filter_env.step(&c.filter_env);
            let e_mod = self.mod_env.step(&c.mod_env);
            let eg = self.eg.step(&c.eg) * depth;
            // m = clamp(smoothed(v15 + MW) + (2v19 − 1)·E_mod + [EG→Osc.Mod] + [LFO→MOD], 0, 1).
            let m = (amount[i] + c.mod_contour * e_mod + c.eg_mod * eg + c.lfo_mod * lfo[i])
                .clamp(0.0, 1.0);
            let active = m > MOD_THRESHOLD;
            let (fm, ring) = (active && c.mod_kind == FM, active && c.mod_kind == RING);
            let (mut left, mut right, mut b0) = (0.0, 0.0, 0.0);
            // The wavetable positions glide to their targets, and the readers follow them sample by sample.
            self.position[0] += POSITION_SMOOTHING * (c.pos_a - self.position[0]);
            self.position[1] += POSITION_SMOOTHING * (c.pos_b - self.position[1]);
            let (table_a, table_b) = (
                wave_a.reader(self.table_a, self.position[0]),
                wave_b.reader(self.table_b, self.position[1]),
            );
            for k in 0..c.copies {
                let b = table_b.read(self.phase_b[k], dither);
                // FM: A is read at φ_A + (B + 1)·m cycles.
                let phase_a = if fm {
                    self.phase_a[k].wrapping_add(cycles((b + 1.0) * m * c.fm_scale))
                } else {
                    self.phase_a[k]
                };
                let mut a = table_a.read(phase_a, dither);
                if ring {
                    a *= 1.0 + 3.0 * m * b;
                }
                let x = c.gain_a * a + c.gain_b * b;
                left += x * c.copy_pan[k][0];
                right += x * c.copy_pan[k][1];
                if k == 0 {
                    b0 = b;
                }
                // Sync: A's phase resets to 0 whenever B's phase wraps.
                let (phase_b, wrapped) = self.phase_b[k].overflowing_add(self.step_b[k]);
                self.phase_b[k] = phase_b;
                self.phase_a[k] = if sync && wrapped {
                    0
                } else {
                    self.phase_a[k].wrapping_add(self.step_a[k])
                };
            }
            let noise = c.noise * white(&mut self.noise);
            // c = clamp(S + (2v27 − 1)³·E_flt + [EG→Filter] + [LFO→FLT] + [Filter mod], 0, 1).
            self.s += CUTOFF_SMOOTHING * (self.s_target - self.s);
            let filter_mod = if active && c.mod_kind == FILTER {
                0.5 * m * b0
            } else {
                0.0
            };
            let cutoff = (self.s
                + c.filter_contour * e_filter
                + c.eg_filter * eg
                + c.lfo_filter * lfo[i]
                + filter_mod)
                .clamp(0.0, 1.0);
            self.last_cutoff = cutoff;
            let (a, b) = filter.at(cutoff);
            // DRIVE: the voice is boosted into a soft clip before the filter ([`drive_clip`]), so it gets fuller and
            // grittier; the output gives part of the level back. Off (0) leaves the signal bit for bit as it was.
            let (left, right, give_back) = if c.drive > 0.0 {
                let gain = 1.0 + (DRIVE_GAIN - 1.0) * c.drive;
                let (l, r) = (drive_clip(left * gain), drive_clip(right * gain));
                (l, r, 1.0 / (1.0 + 1.5 * c.drive))
            } else {
                (left, right, 1.0)
            };
            let (out_l, out_r) = if c.filter_extra == EXTRA_COMB {
                // Comb: the delay follows the cutoff's frequency, the resonance sets the feedback.
                let hz = CUTOFF_HZ * (CUTOFF_OCTAVES * cutoff).exp2();
                let delay = c.sample_rate / hz;
                let g = COMB_MIN_FEEDBACK + (COMB_MAX_FEEDBACK - COMB_MIN_FEEDBACK) * c.resonance;
                let out_l = self.comb.process(0, left + noise, delay, g);
                let out_r = self.comb.process(1, right + noise, delay, g);
                self.comb.advance();
                (out_l, out_r)
            } else {
                let kind = match c.filter_extra {
                    EXTRA_BP => KIND_BP,
                    EXTRA_NOTCH => KIND_NOTCH,
                    _ => c.filter_kind,
                };
                let feedback = c.resonance * b;
                (
                    ladder(&mut self.stages[0], left + noise, a, feedback, kind),
                    ladder(&mut self.stages[1], right + noise, a, feedback, kind),
                )
            };
            let (out_l, out_r) = (out_l * give_back, out_r * give_back);
            // voice = filter out × amp env × gain × pan; gain = clamp(velocity + slide velocity, 0, 1).
            self.gain += SMOOTHING * (velocity - self.gain);
            let gain = amp * self.gain;
            bus[0][i] += out_l * gain * self.pan[0];
            bus[1][i] += out_r * gain * self.pan[1];
            // Idle once the amp env is below 10⁻⁴ while aiming at 0: released, or decaying to sustain 0.
            let to_zero = self.amp.stage == Stage::Release
                || (self.amp.stage == Stage::Decay && c.amp.sustain == 0.0);
            if to_zero && amp < FREE_LEVEL {
                self.amp.stage = Stage::Idle;
                return;
            }
        }
    }
}

/// One channel of the filter: u = clip(x − v22·b·y8[n−1]), eight stages y += a·(u − y). LP 12 is stage 4, LP 24
/// stage 8, HP the clipped input minus stage 1.
fn ladder(y: &mut [f32; STAGES], x: f32, a: f32, feedback: f32, kind: u8) -> f32 {
    let u = x - feedback * y[STAGES - 1];
    let u = u / (1.0 + CLIP * u.abs());
    let mut input = u;
    for stage in y.iter_mut() {
        *stage += a * (input - *stage);
        input = *stage;
    }
    match kind {
        0 => y[3],
        1 => y[STAGES - 1],
        // BP: stage 2 minus stage 6 (peaks near the cutoff, 6 dB/oct below it, steeper above).
        KIND_BP => y[1] - y[5],
        // Notch: LP 4th order plus HP 1st order (a deep dip just below the cutoff, flat elsewhere).
        KIND_NOTCH => y[3] + (u - y[0]),
        _ => u - y[0],
    }
}

/// The `ladder` kinds of the extra filter types.
const KIND_BP: u8 = 3;
const KIND_NOTCH: u8 = 4;

/// The `Comb` type's feedback at resonance 0 and 1 (a comb needs some feedback to ring).
const COMB_MIN_FEEDBACK: f32 = 0.5;
const COMB_MAX_FEEDBACK: f32 = 0.95;
/// Samples per delay line: the lowest comb pitch (32 Hz) at up to 128 kHz.
const COMB_LEN: usize = 4096;

/// A voice's stereo comb: one feedback delay line per channel, read with a fractional delay.
#[derive(Clone, Copy)]
struct CombLines {
    buf: [[f32; COMB_LEN]; 2],
    pos: usize,
}

impl Default for CombLines {
    fn default() -> Self {
        Self {
            buf: [[0.0; COMB_LEN]; 2],
            pos: 0,
        }
    }
}

impl CombLines {
    /// y = x + g·y[n − delay] (linear interpolation); the output is scaled by √(1 − g) to keep the level close to the
    /// other filter types. Call [`Self::advance`] once per sample, after both channels.
    fn process(&mut self, channel: usize, x: f32, delay: f32, g: f32) -> f32 {
        let delay = delay.clamp(2.0, (COMB_LEN - 2) as f32);
        let at = (self.pos + COMB_LEN) as f32 - delay;
        let i = at as usize;
        let frac = at - i as f32;
        let line = &mut self.buf[channel];
        let (a, b) = (line[i % COMB_LEN], line[(i + 1) % COMB_LEN]);
        let mut y = x + g * (a + (b - a) * frac);
        // Keeps silence silent: no denormals ringing in the line.
        if y.abs() < 1e-20 {
            y = 0.0;
        }
        line[self.pos] = y;
        y * (1.0 - g).sqrt()
    }

    fn advance(&mut self) {
        self.pos = (self.pos + 1) % COMB_LEN;
    }
}

/// The 32-bit phase step of `freq`·`ratio` Hz.
fn phase_step(freq: f32, ratio: f64, sample_rate: f32) -> u32 {
    (f64::from(freq) * ratio / f64::from(sample_rate) * 4_294_967_296.0) as u64 as u32
}

/// `x` cycles as a 32-bit phase offset, wrapped.
fn cycles(x: f32) -> u32 {
    (f64::from(x) * 4_294_967_296.0) as i64 as u32
}

/// Xorshift32.
fn next(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

/// White noise, uniform in [−1, 1).
fn white(state: &mut u32) -> f32 {
    (next(state) >> 8) as f32 / (1 << 23) as f32 - 1.0
}

/// GMSynth. Build it on the app thread; every [`Instrument`] method is real-time safe.
pub struct GmSynth {
    sample_rate: f32,
    params: [f32; PARAMS],
    voices: [Voice; VOICES],
    /// The key table: keys down, for MONO's fallback and AUTO.
    held: [bool; 128],
    /// The previously played key, where the next note's glide starts (none after reset).
    last_key: Option<u8>,
    serial: u64,
    wave_a: Option<Source>,
    wave_b: Option<Source>,
    filter: &'static FilterTable,
    /// Channel Pitch Bend v and mod wheel (CC #1) v.
    bend: f32,
    wheel: f32,
    /// `CSTM`: PB RANGE R in semitones, MW AMOUNT, MW DEST item.
    pb_range: f32,
    /// `CSTM`: the extra filter type ([`FILTER_EXTRA`]).
    filter_extra: u8,
    filter_drive: f32,
    /// `CSTM`: the FM range item ([`FM_RANGE`]).
    fm_range: usize,
    /// `CSTM`: the LFO's mode and the random range ([`LFO_MODE`]), the value held now and the phase last seen.
    lfo_mode: u8,
    lfo_low: f32,
    lfo_high: f32,
    lfo_held: f32,
    lfo_last_phase: f64,
    /// `CSTM`: the LFO sync item ([`LFO_SYNC`]), whether notes restart the LFO ([`LFO_KEYSYNC`]), the beat position it
    /// counts its cycles from (the song's, while it plays) and the cycles at the last sample.
    lfo_sync: usize,
    lfo_keysync: bool,
    /// `CSTM`: the free modulation routes ([`MOD_BASE`]).
    routes: [Route; MOD_SLOTS],
    lfo_origin: f64,
    lfo_cycles: f64,
    /// The panel's live values ([`LIVE_LEN`]) and the probe that makes the spectrum in them.
    live: Box<[f32]>,
    probe: SpectrumProbe,
    mw_amount: f32,
    mw_dest: usize,
    control: Control,
    /// Samples into the current block.
    block_pos: usize,
    /// The LFO phase in cycles: free-running while stopped, else the song's.
    lfo_phase: f64,
    /// Smoothed v15 + MW, master level and pan; `snap` sets them to their targets at the next block (after reset).
    mod_amount: f32,
    level: f32,
    pan: f32,
    snap: bool,
    /// Dither and random phases.
    rng: u32,
    /// FX 1–3: a processor bank each, and the kind each enables this block (none while its Enable is 0).
    fx: [Bank; FX_SLOTS],
    fx_kinds: [Option<usize>; FX_SLOTS],
}

impl GmSynth {
    /// A GMSynth at `sample_rate` (44.1 kHz if that is not positive) with the default params and settings. Builds
    /// the default shape and the filter table on first use.
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            TABLE_RATE as f32
        };
        let mut synth = Self {
            sample_rate,
            params: DEFAULTS,
            voices: [Voice::default(); VOICES],
            held: [false; 128],
            last_key: None,
            serial: 0,
            wave_a: shape(DEFAULT_SHAPE).map(Source::Shape),
            wave_b: shape(DEFAULT_SHAPE).map(Source::Shape),
            filter: FILTER_TABLE.get_or_init(FilterTable::build),
            bend: 0.5,
            wheel: 0.0,
            pb_range: PB_RANGES[1],
            filter_extra: 0,
            filter_drive: 0.0,
            fm_range: 0,
            lfo_mode: 0,
            lfo_low: 0.0,
            lfo_high: 1.0,
            lfo_held: 0.0,
            lfo_last_phase: 0.0,
            lfo_sync: 0,
            lfo_keysync: false,
            routes: [Route::default(); MOD_SLOTS],
            lfo_origin: 0.0,
            lfo_cycles: 0.0,
            live: vec![0.0; LIVE_LEN].into(),
            probe: SpectrumProbe::new(sample_rate),
            mw_amount: 0.5,
            mw_dest: MW_OSC_MOD,
            control: Control::default(),
            block_pos: 0,
            lfo_phase: 0.0,
            mod_amount: 0.0,
            level: 0.0,
            pan: 0.5,
            snap: true,
            rng: SEED,
            fx: std::array::from_fn(|_| Bank::new(sample_rate)),
            fx_kinds: [None; FX_SLOTS],
        };
        synth.reset();
        synth
    }

    fn beats_per_frame(&self, context: &ProcessContext) -> f64 {
        f64::from(context.tempo_bpm.max(0.0)) / (60.0 * f64::from(self.sample_rate))
    }

    /// A new random LFO value between the range's ends (−1…1 over 0…1 each).
    fn random_lfo(&mut self) -> f32 {
        let u = (next(&mut self.rng) >> 8) as f32 / (1 << 24) as f32;
        let (low, high) = (
            self.lfo_low.min(self.lfo_high),
            self.lfo_low.max(self.lfo_high),
        );
        -1.0 + 2.0 * (low + (high - low) * u)
    }

    /// The LFO's cycle in beats: the SYNC menu's note value, or with `Orig.` the RATE menu's.
    fn lfo_period(&self) -> f64 {
        f64::from(match self.lfo_sync {
            0 => tempo_step(&GMSYNTH_RATE_BEATS, self.params[32]),
            item => LFO_SYNC_BEATS[(item - 1).min(LFO_SYNC_BEATS.len() - 1)],
        })
    }

    /// The LFO's cycles since `lfo_origin` at `frame` of the call, counted on the song's beats while it plays.
    fn lfo_cycles_at(&self, context: &ProcessContext, frame: usize) -> f64 {
        let beat = context.start_beat + frame as f64 * self.beats_per_frame(context);
        beat / self.lfo_period() - self.lfo_origin
    }

    /// The LFO phase at `frame` of the call: the song's while it plays, else the free-running one.
    fn lfo_phase_at(&self, context: &ProcessContext, frame: usize) -> f64 {
        if context.playing {
            self.lfo_cycles_at(context, frame).rem_euclid(1.0)
        } else {
            self.lfo_phase
        }
    }

    /// The block laws, at `frame` of the call.
    fn control(&mut self, context: &ProcessContext, frame: usize) {
        let lfo_shape = ((2.0 * self.params[33]) as u8).min(2);
        let lfo_now = if self.lfo_mode == 0 {
            lfo_value(lfo_shape, self.lfo_phase_at(context, frame))
        } else {
            self.lfo_held
        };
        // Free routes: the LFO or the EG added to a copy of the params, so that every law below, the FX slots'
        // included, reads the modulated value. A copy of 75 floats per 32-sample block, no allocation.
        let mut eff = self.params;
        let eg_now = self
            .voices
            .iter()
            .filter(|voice| voice.sounding())
            .max_by_key(|voice| voice.age)
            .map_or(0.0, |voice| voice.eg.e);
        for route in &self.routes {
            let source = match route.source {
                1 => lfo_now,
                2 => eg_now,
                _ => continue,
            };
            if let (Some(target), true) = (route.target, route.amount != 0.0) {
                let target = &mut eff[MOD_TARGETS[target]];
                *target = (*target + route.amount * source).clamp(0.0, 1.0);
            }
        }
        // FX LFOs: frac(song beats / R_eff) at the block start while the song plays, else free-running. The FX block
        // is this 32-sample control block.
        let start_beat = if context.playing {
            context.start_beat + frame as f64 * self.beats_per_frame(context)
        } else {
            -1.0
        };
        for (slot, (bank, kind)) in self.fx.iter_mut().zip(&mut self.fx_kinds).enumerate() {
            let settings;
            (*kind, settings) = fx_slot(&eff, slot);
            bank.prepare_block(&settings, context.tempo_bpm, start_beat, BLOCK);
        }
        let v = &eff;
        let sr = self.sample_rate;
        let c = &mut self.control;
        let mw = self.wheel * self.mw_amount;
        let mw_at = |dest: usize| if self.mw_dest == dest { mw } else { 0.0 };
        let (mw_filter, mw_lfo, mw_osc) = (mw_at(MW_FILTER), mw_at(MW_LFO), mw_at(MW_OSC_MOD));

        c.amp = Rates {
            attack: attack_rate(v[3], sr),
            decay: decay_rate(v[4], sr),
            sustain: v[5] * v[5],
            release: RATE_SCALE / (sr * (v[6] + 0.001)),
        };
        c.mod_env = Rates {
            attack: attack_rate(v[17], sr),
            decay: decay_rate(v[18], sr),
            ..Rates::default()
        };
        c.filter_env = Rates {
            attack: attack_rate(v[25], sr),
            decay: decay_rate(v[26], sr),
            ..Rates::default()
        };
        c.eg = Rates {
            attack: attack_rate(v[36], sr),
            decay: decay_rate(v[37], sr),
            sustain: v[38] * v[38],
            release: RATE_SCALE / (sr * (200.0 * v[39].powi(4) + 0.001)),
        };

        // Choir: N = int(3·v42) + 1; copy i at 2^((D·i/(N−1) − D/2)/12), odd i ×1.5 (Chord) or ×2 (Octave) by
        // int(2·v43); x_i = v45·(2i/(N−1) − 1), L = clamp(1 − x), R = clamp(1 + x). N = 1 sits at the centre, as its
        // x = 0, and g = 0.5·(1 − v20²).
        c.copies = ((3.0 * v[42]) as usize + 1).min(COPIES);
        let spread = v[44];
        let scale = [1.0, 1.5, 2.0][((2.0 * v[43]) as usize).min(2)];
        for k in 0..COPIES {
            let t = if c.copies > 1 {
                k as f32 / (c.copies - 1) as f32
            } else {
                0.5
            };
            let odd = if k % 2 == 1 { scale } else { 1.0 };
            c.ratio[k] = f64::from(((spread * t - spread / 2.0) / 12.0).exp2()) * odd;
            let x = v[45] * (2.0 * t - 1.0);
            c.copy_pan[k] = [(1.0 - x).clamp(0.0, 1.0), (1.0 + x).clamp(0.0, 1.0)];
        }
        // g = 0.5/(1 + 0.15·(N − 1)) × (1 − v20²); A g·(1 − |2v13 − 1|), B g·(2v13 − 1); noise v20².
        let g = 0.5 / (1.0 + 0.15 * (c.copies - 1) as f32) * (1.0 - v[20] * v[20]);
        c.gain_a = g * (1.0 - (2.0 * v[13] - 1.0).abs());
        c.gain_b = g * (2.0 * v[13] - 1.0);
        c.noise = v[20] * v[20];
        c.ratio_b = ratio_b(v);
        c.offset = transpose_step(v[28]) + self.pb_range * (2.0 * self.bend - 1.0);
        c.slide = v[29];

        // EG depth c = 2v40 − 1 by int(2·v41): Filter c³, Osc. Mod |c|·c, Pitch 12·c·|c| st.
        let depth = 2.0 * v[40] - 1.0;
        let eg_dest = ((2.0 * v[41]) as usize).min(2);
        c.eg_filter = if eg_dest == 0 { depth.powi(3) } else { 0.0 };
        c.eg_mod = if eg_dest == 1 {
            depth.abs() * depth
        } else {
            0.0
        };
        c.eg_pitch = if eg_dest == 2 {
            12.0 * depth * depth.abs()
        } else {
            0.0
        };
        // VEL: any nonzero value is on.
        c.velocity_depth = v[74] != 0.0;

        // LFO: A = 2v34 − 1, MW→LFO adds to |A|, clamp ±1; depth by int(4·v35): FLT A³, MOD |A|A, FREQ 12·A|A| st,
        // LVL ±12·A dB, PAN |A|A.
        let a = 2.0 * v[34] - 1.0;
        let a = (a.abs() + mw_lfo).min(1.0).copysign(a);
        c.lfo_shape = lfo_shape;
        let lfo_dest = ((4.0 * v[35]) as usize).min(4);
        let at = |dest: usize, value: f32| if lfo_dest == dest { value } else { 0.0 };
        c.lfo_filter = at(0, a.powi(3));
        c.lfo_mod = at(1, a.abs() * a);
        c.lfo_mod_now = c.lfo_mod * lfo_now;
        c.lfo_pitch = at(2, 12.0 * a * a.abs()) * lfo_now;
        let lfo_level = at(3, 12.0 * a) * lfo_now;
        let lfo_pan = at(4, a.abs() * a) * lfo_now;

        c.mod_kind = ((3.0 * v[16]) as u8).min(3);
        c.mod_contour = 2.0 * v[19] - 1.0;
        c.mod_target = v[15] + mw_osc;
        c.sync_base = c.mod_target.clamp(0.0, 1.0);

        c.filter_contour = (2.0 * v[27] - 1.0).powi(3);
        c.cutoff = v[21];
        c.kbd = v[24];
        c.mw_filter = mw_filter;
        c.resonance = v[22];
        c.filter_kind = ((2.0 * v[23]) as u8).min(2);
        c.filter_extra = self.filter_extra;
        c.drive = self.filter_drive;
        c.source_a = SourceKind::of(&self.wave_a);
        c.source_b = SourceKind::of(&self.wave_b);
        c.pos_a = v[POS_A];
        c.pos_b = v[POS_B];
        c.fm_scale = FM_SCALES[self.fm_range];
        c.sample_rate = sr;

        // Master: level dB = 66·v1 − 48 (+ LFO), 0 below −60 dB; P = clamp(v2 + LFO, 0, 1).
        let db = 66.0 * v[1] - 48.0 + lfo_level;
        c.level = if db < SILENT_DB {
            0.0
        } else {
            10.0_f32.powf(db / 20.0)
        };
        c.pan = (v[2] + lfo_pan).clamp(0.0, 1.0);
        if self.snap {
            (self.level, self.pan, self.mod_amount) = (c.level, c.pan, c.mod_target);
            self.snap = false;
        }

        for voice in self.voices.iter_mut().filter(|voice| voice.sounding()) {
            voice.glide(self.control.slide, sr);
            voice.control(&self.control, sr);
        }
    }

    /// Renders `left`/`right`, which lie inside one block, starting at `frame` of the call.
    fn render(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        context: &ProcessContext,
        frame: usize,
    ) {
        let n = left.len();
        let c = self.control;
        let (mut dither, mut lfo, mut amount) =
            ([0_u32; BLOCK], [0.0_f32; BLOCK], [0.0_f32; BLOCK]);
        let step = self.beats_per_frame(context) / self.lfo_period();
        for i in 0..n {
            // One dither per sample, shared by both oscillators, every copy and every voice.
            dither[i] = next(&mut self.rng) >> 10;
            let phase = self.lfo_phase_at(context, frame + i);
            self.lfo_phase = (phase + step).fract();
            lfo[i] = match self.lfo_mode {
                0 => lfo_value(c.lfo_shape, phase),
                mode => {
                    // S&H: a new value when the phase wraps (a new cycle); Note: held until the next note.
                    if mode == LFO_SAMPLE_HOLD && phase < self.lfo_last_phase {
                        self.lfo_held = self.random_lfo();
                    }
                    self.lfo_held
                }
            };
            self.lfo_last_phase = phase;
            if context.playing {
                self.lfo_cycles = self.lfo_cycles_at(context, frame + i);
            }
            // What the panel shows of the LFO: its value and phase at the block's last sample.
            self.live[LIVE_LFO] = lfo[i];
            self.live[LIVE_PHASE] = phase as f32;
            self.mod_amount += SMOOTHING * (c.mod_target - self.mod_amount);
            amount[i] = self.mod_amount;
        }
        let mut bus = [[0.0_f32; BLOCK]; 2];
        // A missing shape for A or B silences the voices; they hold until it is back.
        if let (Some(a), Some(b)) = (&self.wave_a, &self.wave_b) {
            for voice in self.voices.iter_mut().filter(|voice| voice.sounding()) {
                let (dither, lfo, amount) = (&dither[..n], &lfo[..n], &amount[..n]);
                voice.render(&c, (a, b), self.filter, dither, lfo, amount, &mut bus);
            }
        }
        for i in 0..n {
            // FX 1 → FX 2 → FX 3 in series, then the master level and pan.
            let mut out = [bus[0][i], bus[1][i]];
            for (bank, &kind) in self.fx.iter_mut().zip(&self.fx_kinds) {
                out = bank.process_frame(out, kind);
            }
            self.level += SMOOTHING * (c.level - self.level);
            self.pan += SMOOTHING * (c.pan - self.pan);
            let (l, r) = ((2.0 - 2.0 * self.pan).min(1.0), (2.0 * self.pan).min(1.0));
            let (out_left, out_right) = (out[0] * self.level * l * l, out[1] * self.level * r * r);
            left[i] += out_left;
            right[i] += out_right;
            // The spectrum of the synth's own output, for the filter's picture.
            self.probe
                .push(0.5 * (out_left + out_right), &mut self.live[LIVE_BANDS..]);
        }
        // The cutoff the sounding voices have now, for the filter's picture to move with.
        let sounding = self.voices.iter().filter(|voice| voice.sounding());
        let (sum, count) =
            sounding.fold((0.0, 0), |(sum, n), voice| (sum + voice.last_cutoff, n + 1));
        self.live[LIVE_CUTOFF] = if count > 0 {
            sum / count as f32
        } else {
            self.params[21]
        };
    }
}

/// FX slot `slot`'s enabled kind and settings. Enable is on at any v ≠ 0, 0.5 included. Kind
/// and Sync trunc(9v) (DIST … VOX; R = [1/4 … 32] beats), LFO amount sign(2v − 1)·(2v − 1)², shape trunc(2v) (≥ 2
/// Square), X and Y raw.
fn fx_slot(v: &[f32; PARAMS], slot: usize) -> (Option<usize>, Settings) {
    let p = &v[FX_PARAMS + 7 * slot..][..7];
    let menu = |v: f32, n: usize| ((v.clamp(0.0, 1.0) * n as f32) as usize).min(n);
    let amount = 2.0 * p[4] - 1.0;
    let settings = Settings {
        x: p[5],
        y: p[6],
        rate_beats: GMSYNTH_RATE_BEATS[menu(p[2], 9)],
        amount: amount * amount.abs(),
        shape: menu(p[3], 2),
    };
    ((p[1] != 0.0).then_some(menu(p[0], 9)), settings)
}

/// f_B/f_A for PITCH B ±24 st and FINE B ±0.5 st, all in f64: s = trunc(f64(v11)·48 − 24), exact, then
/// f32(exp2(((f64(v12) − 0.5) + s)·(1/12))). The `%d` text shows this step too ([`pitch_b_step`]), so the text always
/// matches the sound.
fn ratio_b(v: &[f32; PARAMS]) -> f32 {
    let s = pitch_b_step(v[11]);
    (((f64::from(v[12]) - 0.5) + s) * (1.0 / 12.0)).exp2() as f32
}

/// Oscillator B's frequency over A's from PITCH B (`v11`) and FINE B (`v12`), as the engine uses it.
pub fn osc_b_ratio(v11: f32, v12: f32) -> f32 {
    (((f64::from(v12) - 0.5) + pitch_b_step(v11)) * (1.0 / 12.0)).exp2() as f32
}

/// PITCH B (p11) in semitones: s = trunc(f64(v)·48 − 24), exact (the Slicer law).
pub fn pitch_b_step(v: f32) -> f64 {
    (f64::from(v) * 48.0 - 24.0).trunc()
}

/// TRNSP: s = trunc(fma_f32(v28, 48, −24)), one fused multiply-add then truncation (the DW Sampler law).
/// The `%d` text uses it too.
pub fn transpose_step(v: f32) -> f32 {
    48.0_f32.mul_add(v, -24.0).trunc()
}

/// LFO shapes by int(2·v33): Sine sin 2πφ, Saw 2φ − 1, Square −1 below φ = 0.5, else +1.
fn lfo_value(shape: u8, phase: f64) -> f32 {
    match shape {
        0 => (TAU * phase as f32).sin(),
        1 => (2.0 * phase - 1.0) as f32,
        _ => {
            if phase < 0.5 {
                -1.0
            } else {
                1.0
            }
        }
    }
}

/// The nominal cutoff frequency in Hz of the normalised cutoff `c`: 32·2^(9.27c).
pub fn cutoff_hz(c: f32) -> f32 {
    CUTOFF_HZ * (CUTOFF_OCTAVES * c.clamp(0.0, 1.0)).exp2()
}

/// The normalised cutoff of a frequency in Hz, the inverse of [`cutoff_hz`].
pub fn cutoff_of_hz(hz: f32) -> f32 {
    ((hz.max(1.0) / CUTOFF_HZ).log2() / CUTOFF_OCTAVES).clamp(0.0, 1.0)
}

/// The filter's gain in dB at each of `freqs` (Hz), for a display: the cutoff `v21`, the resonance `v22`, KIND `v23`
/// and the `EXTRA` type (0 off, 1 BP, 2 Notch, 3 Comb), at the sample rate the filter table is authored for. The linear
/// response of the ladder (eight one-pole stages and the feedback from the last, one sample late, taps as
/// [`ladder`] reads them); the input soft clip and the envelopes and LFO that move the cutoff are not in it.
pub fn filter_response(
    cutoff: f32,
    resonance: f32,
    kind: u8,
    extra: u8,
    freqs: &[f32],
) -> Vec<f32> {
    use std::f64::consts::TAU as TAU64;
    let table = FILTER_TABLE.get_or_init(FilterTable::build);
    let (a, b) = table.at(cutoff.clamp(0.0, 1.0));
    let (a, feedback) = (f64::from(a), f64::from(resonance.clamp(0.0, 1.0) * b));
    let rate = TABLE_RATE;
    let comb_g = f64::from(
        COMB_MIN_FEEDBACK + (COMB_MAX_FEEDBACK - COMB_MIN_FEEDBACK) * resonance.clamp(0.0, 1.0),
    );
    let comb_delay = rate / f64::from(cutoff_hz(cutoff));
    freqs
        .iter()
        .map(|&hz| {
            let w = TAU64 * f64::from(hz) / rate;
            let z = Complex::new(w.cos(), -w.sin());
            let gain = if extra == EXTRA_COMB {
                // y = x + g·y[n − D], scaled by √(1 − g).
                let loop_gain = Complex::from_polar(comb_g, -w * comb_delay);
                (1.0 - comb_g).sqrt() / Complex::new(1.0, 0.0).minus(loop_gain).abs()
            } else {
                // One stage: a / (1 − (1 − a) z⁻¹).
                let g = Complex::new(a, 0.0).over(Complex::new(1.0, 0.0).minus(z.scale(1.0 - a)));
                let powers = |n: i32| (0..n).fold(Complex::new(1.0, 0.0), |p, _| p.times(g));
                let tap = match (extra, kind) {
                    (EXTRA_BP, _) => powers(2).minus(powers(6)),
                    (EXTRA_NOTCH, _) => powers(4).plus(Complex::new(1.0, 0.0)).minus(g),
                    (_, 0) => powers(4),
                    (_, 1) => powers(8),
                    _ => Complex::new(1.0, 0.0).minus(g),
                };
                // u = x / (1 + feedback·z⁻¹·G⁸).
                let denominator = Complex::new(1.0, 0.0).plus(z.times(powers(8)).scale(feedback));
                tap.over(denominator).abs()
            };
            (20.0 * gain.max(1e-6).log10()) as f32
        })
        .collect()
}

/// A complex number for [`filter_response`].
#[derive(Clone, Copy)]
struct Complex {
    re: f64,
    im: f64,
}

impl Complex {
    fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    fn from_polar(r: f64, angle: f64) -> Self {
        Self::new(r * angle.cos(), r * angle.sin())
    }

    fn plus(self, o: Self) -> Self {
        Self::new(self.re + o.re, self.im + o.im)
    }

    fn minus(self, o: Self) -> Self {
        Self::new(self.re - o.re, self.im - o.im)
    }

    fn times(self, o: Self) -> Self {
        Self::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }

    fn over(self, o: Self) -> Self {
        let d = o.re * o.re + o.im * o.im;
        Self::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }

    fn scale(self, k: f64) -> Self {
        Self::new(self.re * k, self.im * k)
    }

    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
}

/// Which GMSynth envelope [`envelope_preview`] draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeKind {
    /// LEVEL: attack `v3`, decay `v4`, sustain `v5` (squared), release `v6`.
    Level,
    /// The filter envelope: attack `v25`, decay `v26`; it ignores note-off and decays to 0.
    Filter,
    /// The osc mod envelope: attack `v17`, decay `v18`; as the filter's.
    Mod,
    /// EG: attack `v36`, decay `v37`, sustain `v38` (squared), release `v39`.
    Eg,
}

/// An envelope for a display: its curve over a width split into a share per stage that follows the stage's time loosely
/// (so a 2 ms attack and a 20 s decay are both readable, and the picture moves with the knobs), and the real times for the
/// labels.
#[derive(Clone, Debug, PartialEq)]
pub struct EnvelopePreview {
    /// 0…1, evenly spaced over the display's width.
    pub values: Vec<f32>,
    /// Where the stages end as fractions of the width: attack, decay, sustain hold, release. An envelope that ignores
    /// note-off (the filter's, the osc mod's) ends at its decay and has no hold or release: its last three are 1.
    pub ends: [f32; 4],
    /// The real time of the attack, of the decay until it has settled, and of the release, in seconds.
    pub seconds: [f32; 3],
    /// Whether there is a note-off (a release).
    pub releases: bool,
    /// The sustain level the decay settles at.
    pub sustain: f32,
}

/// The weight of a stage of no length: a hair of width, so that a stage with no time is a steep, visible line and not a
/// jump with nothing to see.
const ENV_MIN_WEIGHT: f32 = 0.04;
/// The weight of the sustain hold, between decay and release.
const ENV_HOLD_WEIGHT: f32 = 0.4;
/// A decay or release is drawn until it is within 0.5 % of where it goes (ln 200 time constants).
const ENV_SETTLE: f32 = 5.298_317;

/// A stage's weight for its share of the display, from its time: nothing but a hair ([`ENV_MIN_WEIGHT`]) at no time,
/// growing with the logarithm of 1 + time/10 ms to a full 1 at 20 s. A stage the engine runs in a millisecond (an
/// attack or decay at 0 %) is therefore a near-vertical edge, a longer one visibly wider, and the picture moves with
/// ATTACK, DECAY and RELEASE.
fn env_weight(seconds: f32) -> f32 {
    let g = (1.0 + seconds.max(0.0) / 0.01).log10() / (1.0 + 20.0_f32 / 0.01).log10();
    ENV_MIN_WEIGHT + g.clamp(0.0, 1.0)
}

/// The envelope `kind` of params `v`: the engine's own one-pole curves (attack aims at 5.0 and ends at 1, decay and
/// release are exponentials), drawn stage by stage over fixed shares of `points` evenly spaced points, with the real
/// stage times beside them. The times follow the engine's laws (`attack_rate`, `decay_rate`, the release rates).
pub fn envelope_preview(kind: EnvelopeKind, v: &[f32], points: usize) -> EnvelopePreview {
    let get = |i: usize| v.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    // The time constants in seconds: k = RATE_SCALE/(sr·t) per sample, so τ = t/RATE_SCALE.
    // Attack: k = RATE_SCALE/(sr·100v⁴), and it ends where 5·(1 − e^(−kn)) = 1, at kn = ln 1.25.
    let attack_s = |v: f32| 0.223_143_55 * 100.0 * v.powi(4) / RATE_SCALE;
    let (attack, decay, sustain, release) = match kind {
        EnvelopeKind::Level => (get(3), get(4), get(5) * get(5), Some(get(6) + 0.001)),
        EnvelopeKind::Filter => (get(25), get(26), 0.0, None),
        EnvelopeKind::Mod => (get(17), get(18), 0.0, None),
        EnvelopeKind::Eg => (
            get(36),
            get(37),
            get(38) * get(38),
            Some(200.0 * get(39).powi(4) + 0.001),
        ),
    };
    let seconds = [
        attack_s(attack),
        ENV_SETTLE * (200.0 * decay.powi(7) + 0.001) / RATE_SCALE,
        release.map_or(0.0, |t| ENV_SETTLE * t / RATE_SCALE),
    ];
    // The width of each stage follows its time, loosely ([`env_weight`]).
    let weights = if release.is_some() {
        [
            env_weight(seconds[0]),
            env_weight(seconds[1]),
            ENV_HOLD_WEIGHT,
            env_weight(seconds[2]),
        ]
    } else {
        [env_weight(seconds[0]), env_weight(seconds[1]), 0.0, 0.0]
    };
    // Shares of a fixed whole, the width the longest stages would need: a stage of no time keeps almost none of it (a
    // vertical edge) and the picture ends early, with a flat line after it, when every stage is short.
    let full = if release.is_some() {
        3.0 * (1.0 + ENV_MIN_WEIGHT) + ENV_HOLD_WEIGHT
    } else {
        2.0 * (1.0 + ENV_MIN_WEIGHT)
    };
    let mut ends = [0.0; 4];
    let mut total = 0.0;
    for (end, weight) in ends.iter_mut().zip(weights) {
        total += weight / full;
        *end = total;
    }
    // The attack runs 0 → 1 as 5·(1 − 0.8^u) over u in 0…1 (0.8 = 1 − 1/5: the level at which it ends).
    let n = points.max(2);
    let values = (0..n)
        .map(|i| {
            let f = i as f32 / (n - 1) as f32;
            if f <= ends[0] {
                let u = if ends[0] > 0.0 { f / ends[0] } else { 1.0 };
                (ATTACK_TARGET * (1.0 - (1.0 - 1.0 / ATTACK_TARGET).powf(u))).min(1.0)
            } else if f <= ends[1] {
                let u = (f - ends[0]) / (ends[1] - ends[0]);
                sustain + (1.0 - sustain) * (-ENV_SETTLE * u).exp()
            } else if release.is_none() || f <= ends[2] {
                sustain
            } else if f <= ends[3] {
                let u = (f - ends[2]) / (ends[3] - ends[2]);
                (sustain * (-ENV_SETTLE * u).exp() * (1.0 - u)).max(0.0)
            } else {
                0.0
            }
        })
        .collect();
    EnvelopePreview {
        values,
        ends,
        seconds,
        releases: release.is_some(),
        sustain,
    }
}

/// `points` samples (−1…1) of the LFO shape `v33` over `cycles` cycles: Sine, Saw or Square by int(2·v33), as the
/// engine reads them.
pub fn lfo_preview(shape: f32, cycles: f32, points: usize) -> Vec<f32> {
    let shape = ((2.0 * shape.clamp(0.0, 1.0)) as u8).min(2);
    (0..points.max(2))
        .map(|i| {
            let t = f64::from(cycles) * i as f64 / (points.max(2) - 1) as f64;
            lfo_value(shape, t.rem_euclid(1.0))
        })
        .collect()
}

/// The voice pan law: L = 1 − max(0, 2p − 1), R = 1 + min(0, 2p − 1).
fn pan_gains(p: f32) -> [f32; 2] {
    [
        1.0 - (2.0 * p - 1.0).max(0.0),
        1.0 + (2.0 * p - 1.0).min(0.0),
    ]
}

impl Instrument for GmSynth {
    /// The panel's live values ([`LIVE_LFO`], [`LIVE_PHASE`], then the spectrum's bands from [`LIVE_BANDS`]).
    fn analysis(&self) -> &[f32] {
        &self.live
    }

    /// Stores every param; hidden 7–10 and 67–73 are never read. Turning MONO on sends a note-off to all 128
    /// keys first.
    fn set_parameter(&mut self, index: usize, value: f32) {
        if index == SINGLE_VOICE && value != 0.0 && self.params[SINGLE_VOICE] == 0.0 {
            for key in 0..128 {
                self.note_off(key);
            }
        }
        if let Some(param) = self.params.get_mut(index) {
            *param = value;
        }
    }

    /// Ignored while Enable < 0.5. Poly: the highest-numbered free voice, else the oldest-started is stolen.
    /// MONO: voice 0, legato (key and velocity only) while it sounds unreleased. A note arriving
    /// mid-block takes its glide step and block values at once; one at a block start waits for that block's laws.
    fn note_on(&mut self, key: u8, velocity: f32, pan: f32) {
        if self.params[ENABLE] < 0.5 || key > 127 {
            return;
        }
        let others_held = self
            .held
            .iter()
            .enumerate()
            .any(|(k, &down)| down && k != usize::from(key));
        self.held[usize::from(key)] = true;
        let velocity = velocity.clamp(0.0, 1.0);
        if self.lfo_mode == LFO_PER_NOTE {
            self.lfo_held = self.random_lfo();
        }
        if self.lfo_keysync {
            // The LFO starts its cycle with the note: from phase 0 free-running, or from the song's beat as it is now.
            self.lfo_phase = 0.0;
            self.lfo_origin += self.lfo_cycles;
            self.lfo_cycles = 0.0;
        }
        let mono = self.params[SINGLE_VOICE] != 0.0;
        let glide_from = match self.last_key.replace(key) {
            // AUTO (v31, on at any nonzero v): no glide unless another key is held.
            Some(last) if self.params[31] == 0.0 || others_held => f32::from(last),
            _ => f32::from(key),
        };
        if mono && self.voices[0].held() {
            let voice = &mut self.voices[0];
            voice.key = key;
            voice.velocity = velocity;
            return;
        }
        let index = if mono {
            0
        } else {
            (0..VOICES)
                .rev()
                .find(|&i| !self.voices[i].sounding())
                .or_else(|| (0..VOICES).min_by_key(|&i| self.voices[i].age))
                .unwrap_or(0)
        };
        self.serial += 1;
        let ratio_b = f64::from(ratio_b(&self.params));
        let phase_offset = f64::from(self.params[14]);
        let voice = &mut self.voices[index];
        if !voice.sounding() {
            // A voice reset from idle: random A phases, a clear filter, and its gain snaps to the velocity; a voice
            // still sounding keeps its gain and glides 1 % per sample.
            for phase in &mut voice.phase_a {
                *phase = next(&mut self.rng);
            }
            voice.stages = [[0.0; STAGES]; 2];
            voice.comb = CombLines::default();
            voice.gain = velocity;
        }
        // B locks to A at retrigger: φ_B = frac(v14 + φ_A·f_B/f_A).
        for (b, &a) in voice.phase_b.iter_mut().zip(&voice.phase_a) {
            let cycles = phase_offset + f64::from(a) / 4_294_967_296.0 * ratio_b;
            *b = (cycles.fract() * 4_294_967_296.0) as u32;
        }
        let pan = pan.clamp(0.0, 1.0);
        voice.key = key;
        voice.velocity = velocity;
        voice.note_pan = pan;
        voice.pan = pan_gains(pan);
        voice.slide = [0.0; 3];
        voice.age = self.serial;
        voice.p = glide_from;
        // The amp level continues from where it is; the other envelopes restart from 0.
        voice.amp.stage = Stage::Attack;
        voice.filter_env.restart();
        voice.mod_env.restart();
        voice.eg.restart();
        voice.snap = true;
        if self.block_pos != 0 {
            voice.glide(self.control.slide, self.sample_rate);
            voice.control(&self.control, self.sample_rate);
        }
    }

    /// Poly: releases the voices holding `key`. MONO, the sounding key: falls back to the highest held key below
    /// it, else the lowest above, without a retrigger; none held, it releases.
    fn note_off(&mut self, key: u8) {
        let Some(down) = self.held.get_mut(usize::from(key)) else {
            return;
        };
        *down = false;
        if self.params[SINGLE_VOICE] == 0.0 {
            for voice in self.voices.iter_mut().filter(|voice| voice.key == key) {
                if voice.held() {
                    voice.release();
                }
            }
            return;
        }
        let voice = &mut self.voices[0];
        if voice.held() && voice.key == key {
            let held = |k: &u8| self.held[usize::from(*k)];
            match (0..key)
                .rev()
                .find(held)
                .or_else(|| (key + 1..128).find(held))
            {
                Some(fallback) => voice.key = fallback,
                None => voice.release(),
            }
        }
    }

    fn all_notes_off(&mut self) {
        self.held = [false; 128];
        for voice in &mut self.voices {
            voice.release();
        }
    }

    fn reset(&mut self) {
        self.voices = [Voice::default(); VOICES];
        for (seed, voice) in (1..).zip(&mut self.voices) {
            voice.noise = SEED.rotate_left(seed * 8) ^ seed;
        }
        self.held = [false; 128];
        self.last_key = None;
        self.serial = 0;
        self.block_pos = 0;
        self.lfo_phase = 0.0;
        self.probe.reset(&mut self.live[LIVE_BANDS..]);
        self.live[LIVE_LFO] = 0.0;
        self.live[LIVE_PHASE] = 0.0;
        self.snap = true;
        self.rng = SEED;
        for bank in &mut self.fx {
            bank.reset();
        }
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32], context: ProcessContext) {
        let frames = left.len().min(right.len());
        let mut at = 0;
        while at < frames {
            if self.block_pos == 0 {
                self.control(&context, at);
            }
            let n = (BLOCK - self.block_pos).min(frames - at);
            self.render(&mut left[at..at + n], &mut right[at..at + n], &context, at);
            self.block_pos = (self.block_pos + n) % BLOCK;
            at += n;
        }
    }

    /// Pitch Bend: w = 2v − 1 times R; MIDI CC #1: the mod wheel.
    fn set_controller(&mut self, index: usize, value: f32) {
        match index {
            PITCH_BEND => self.bend = value,
            MOD_WHEEL => self.wheel = value,
            _ => {}
        }
    }

    /// Slides add to every voice's pitch (from the next block), velocity gain (smoothed as a velocity is) and pan.
    fn set_slide(&mut self, pitch: f32, pan: f32, velocity: f32) {
        let finite = |value: f32| if value.is_finite() { value } else { 0.0 };
        for voice in &mut self.voices {
            voice.slide = [finite(pitch), finite(pan), finite(velocity)];
            voice.pan = pan_gains((voice.note_pan + voice.slide[1]).clamp(0.0, 1.0));
        }
    }

    /// The `CSTM` settings. A shape is taken from the cache only; [`prepare_setting`] builds it on the app thread.
    fn set_setting(&mut self, key: u32, value: f32) {
        match key {
            WAVE_A | WAVE_B => {
                let slot = value as usize;
                debug_assert!(
                    slot >= SOURCES || cached_source(slot).is_some(),
                    "prepare_setting builds slot {slot} first"
                );
                *if key == WAVE_A {
                    &mut self.wave_a
                } else {
                    &mut self.wave_b
                } = cached_source(slot);
            }
            PB_RANGE => self.pb_range = PB_RANGES[(value as usize).min(PB_RANGES.len() - 1)],
            MW_AMOUNT => self.mw_amount = value.clamp(0.0, 1.0),
            MW_DEST => self.mw_dest = (value as usize).min(MW_OSC_MOD),
            LFO_MODE => {
                let mode = (value as usize).min(usize::from(LFO_PER_NOTE)) as u8;
                if mode != self.lfo_mode {
                    self.lfo_mode = mode;
                    self.lfo_held = self.random_lfo();
                }
            }
            LFO_SYNC => self.lfo_sync = (value as usize).min(LFO_SYNC_BEATS.len()),
            LFO_KEYSYNC => {
                self.lfo_keysync = value >= 0.5;
                if !self.lfo_keysync {
                    self.lfo_origin = 0.0;
                }
            }
            LFO_LOW | LFO_HIGH => {
                let end = value.clamp(0.0, 1.0);
                if key == LFO_LOW {
                    self.lfo_low = end;
                } else {
                    self.lfo_high = end;
                }
                // The value held now stays inside the new range.
                let (low, high) = (
                    self.lfo_low.min(self.lfo_high),
                    self.lfo_low.max(self.lfo_high),
                );
                self.lfo_held = self.lfo_held.clamp(-1.0 + 2.0 * low, -1.0 + 2.0 * high);
            }
            key if (MOD_BASE..MOD_BASE + 10 * MOD_SLOTS as u32).contains(&key) => {
                let slot = ((key - MOD_BASE) / 10) as usize;
                let route = &mut self.routes[slot];
                match (key - MOD_BASE) % 10 {
                    MOD_SOURCE => route.source = (value as usize).min(MOD_SOURCES.len() - 1) as u8,
                    MOD_TARGET => {
                        // Menu item 0 is Off, item n the target n − 1.
                        let item = (value as usize).min(MOD_TARGETS.len());
                        route.target = item.checked_sub(1);
                    }
                    MOD_AMOUNT => {
                        route.amount = if value.is_finite() {
                            (2.0 * value - 1.0).clamp(-1.0, 1.0)
                        } else {
                            0.0
                        }
                    }
                    _ => {}
                }
            }
            FM_RANGE => self.fm_range = (value as usize).min(FM_SCALES.len() - 1),
            FILTER_DRIVE => {
                self.filter_drive = if value.is_finite() {
                    value.clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }
            FILTER_EXTRA => self.filter_extra = (value as usize).min(usize::from(EXTRA_COMB)) as u8,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;
    const STOPPED: ProcessContext = ProcessContext {
        sample_rate: RATE,
        tempo_bpm: 120.0,
        start_beat: 0.0,
        playing: false,
    };

    fn run(synth: &mut GmSynth, frames: usize) {
        let (mut left, mut right) = (vec![0.0; frames], vec![0.0; frames]);
        synth.process(&mut left, &mut right, STOPPED);
    }

    fn keys(synth: &GmSynth) -> Vec<Option<u8>> {
        (synth.voices.iter())
            .map(|voice| voice.sounding().then_some(voice.key))
            .collect()
    }

    #[test]
    fn filter_table_follows_the_recipe() {
        let table = FILTER_TABLE.get_or_init(FilterTable::build);
        // b: 1.888 at the low end, 9.86 at the top.
        assert!((table.0[0][1] - 1.888).abs() < 0.001, "{}", table.0[0][1]);
        let top = table.0[FILTER_TABLE_LEN - 1][1];
        assert!((top - 9.86).abs() < 0.001, "{top}");
        // 8·arg H − ω = −π and b·|H|⁸ = 1 at f_r, for the c the cutoff law maps there.
        for c in [0.0_f32, 0.2, 0.5357, 0.8, 0.99] {
            let (a, b) = table.at(c);
            let f_r =
                f64::from(PEAK_SCALE * CUTOFF_HZ * (CUTOFF_OCTAVES * c).exp2() + PEAK_OFFSET_HZ);
            let w = TAU_64 * f_r / TABLE_RATE;
            let (a, b) = (f64::from(a), f64::from(b));
            let (re, im) = (1.0 - (1.0 - a) * w.cos(), (1.0 - a) * w.sin());
            let phase = 8.0 * -im.atan2(re) - w;
            assert!((phase + PI).abs() < 1e-4, "c {c}: phase {phase}");
            let gain = b * (a / re.hypot(im)).powi(8);
            assert!((gain - 1.0).abs() < 1e-4, "c {c}: loop gain {gain}");
        }
    }

    /// The gain (output RMS over input RMS) of `ladder` kind `kind` for a sine of `w` rad/sample, after it settles.
    fn gain(kind: u8, a: f32, w: f32) -> f32 {
        let mut y = [0.0; STAGES];
        let (mut input, mut output) = (0.0, 0.0);
        for n in 0..40_000 {
            let x = 0.1 * (w * n as f32).sin();
            let out = ladder(&mut y, x, a, 0.0, kind);
            if n >= 20_000 {
                input += x * x;
                output += out * out;
            }
        }
        (output / input).sqrt()
    }

    #[test]
    fn bandpass_passes_the_cutoff_region_and_cuts_both_sides() {
        let (a, _) = FILTER_TABLE.get_or_init(FilterTable::build).at(0.3);
        let wc = -(1.0 - a).ln();
        let at = |ratio: f32| gain(KIND_BP, a, wc * ratio);
        let centre = at(0.47);
        assert!((0.8..=1.3).contains(&centre), "centre {centre}");
        assert!(at(0.01) < 0.1 * centre, "low {}", at(0.01));
        assert!(at(30.0) < 0.1 * centre, "high {}", at(30.0));
    }

    #[test]
    fn notch_cuts_one_spot_and_leaves_the_rest() {
        let (a, _) = FILTER_TABLE.get_or_init(FilterTable::build).at(0.3);
        let wc = -(1.0 - a).ln();
        let at = |ratio: f32| gain(KIND_NOTCH, a, wc * ratio);
        assert!(at(0.6) < 0.1, "dip {}", at(0.6));
        assert!((at(0.01) - 1.0).abs() < 0.1, "low {}", at(0.01));
        assert!((at(30.0) - 1.0).abs() < 0.1, "high {}", at(30.0));
    }

    #[test]
    fn comb_echoes_after_its_delay_with_the_feedback() {
        let mut comb = CombLines::default();
        let (delay, g) = (100.0, 0.5);
        let out: Vec<f32> = (0..250)
            .map(|n| {
                let y = comb.process(0, if n == 0 { 1.0 } else { 0.0 }, delay, g);
                comb.advance();
                y
            })
            .collect();
        let scale = (1.0_f32 - g).sqrt();
        assert!((out[0] - scale).abs() < 1e-6);
        assert!((out[100] - g * scale).abs() < 1e-6, "{}", out[100]);
        assert!((out[200] - g * g * scale).abs() < 1e-6, "{}", out[200]);
        assert!(out[1..100].iter().all(|&y| y == 0.0));
    }

    #[test]
    fn every_extra_filter_type_sounds_and_stays_finite() {
        let render = |extra: f32| {
            let mut synth = GmSynth::new(RATE);
            synth.set_setting(FILTER_EXTRA, extra);
            synth.note_on(60, 1.0, 0.5);
            let (mut left, mut right) = (vec![0.0; 4800], vec![0.0; 4800]);
            synth.process(&mut left, &mut right, STOPPED);
            assert!(left.iter().chain(&right).all(|x| x.is_finite()), "{extra}");
            left
        };
        let off = render(0.0);
        for extra in [1.0, 2.0, 3.0] {
            let out = render(extra);
            assert!(
                out.iter().any(|&x| x.abs() > 1e-4),
                "type {extra} is silent"
            );
            assert!(out != off, "type {extra} sounds like Off");
        }
    }

    #[test]
    fn fm_range_scales_the_fm_depth_and_x1_is_the_original() {
        let render = |range: Option<f32>| {
            let mut synth = GmSynth::new(RATE);
            synth.set_parameter(16, 0.4); // KIND: FM
            synth.set_parameter(15, 0.2); // MOD. AMNT: 20 %
            if let Some(range) = range {
                synth.set_setting(FM_RANGE, range);
            }
            synth.note_on(60, 1.0, 0.5);
            let (mut left, mut right) = (vec![0.0; 4800], vec![0.0; 4800]);
            synth.process(&mut left, &mut right, STOPPED);
            assert!(left.iter().all(|x| x.is_finite()));
            left
        };
        // x1 is what a song without the setting plays; the larger ranges sound different.
        assert_eq!(render(None), render(Some(0.0)));
        for range in [1.0, 2.0, 3.0] {
            assert!(render(Some(range)) != render(None), "x{range}");
        }
        assert_eq!(osc_b_ratio(0.5, 0.5), 1.0);
        assert_eq!(osc_b_ratio(0.75, 0.5), 2.0);
    }

    #[test]
    fn the_filter_response_matches_the_ladder_the_engine_runs() {
        let table = FILTER_TABLE.get_or_init(FilterTable::build);
        let c = 0.4;
        let (a, _) = table.at(c);
        let wc = -(1.0 - a).ln();
        // The time-domain gain of `ladder` at a few frequencies against the display's, both with no resonance.
        let rate = 44_100.0_f64;
        for (kind, extra, name) in [
            (0u8, 0u8, "LP 12"),
            (1, 0, "LP 24"),
            (2, 0, "HP"),
            (0, EXTRA_BP, "BP"),
            (0, EXTRA_NOTCH, "Notch"),
        ] {
            let ladder_kind = match extra {
                EXTRA_BP => KIND_BP,
                EXTRA_NOTCH => KIND_NOTCH,
                _ => kind,
            };
            for ratio in [0.1_f32, 0.5, 1.0, 3.0] {
                let w = wc * ratio;
                let measured = 20.0 * gain(ladder_kind, a, w).log10();
                let hz = (f64::from(w) * rate / std::f64::consts::TAU) as f32;
                let shown = filter_response(c, 0.0, kind, extra, &[hz])[0];
                assert!(
                    (measured - shown).abs() < 1.0,
                    "{name} at {ratio}×: {measured} dB measured, {shown} dB shown"
                );
            }
        }
    }

    /// The time-domain gain of `ladder` kind `kind` with `feedback` for a small sine of `w` rad/sample.
    fn gain_with_feedback(kind: u8, a: f32, feedback: f32, w: f32) -> f32 {
        let mut y = [0.0; STAGES];
        let (mut input, mut output) = (0.0, 0.0);
        for n in 0..60_000 {
            let x = 0.01 * (w * n as f32).sin();
            let out = ladder(&mut y, x, a, feedback, kind);
            if n >= 40_000 {
                input += x * x;
                output += out * out;
            }
        }
        (output / input).sqrt()
    }

    #[test]
    fn the_filter_response_shows_the_resonance_the_engine_has() {
        let table = FILTER_TABLE.get_or_init(FilterTable::build);
        let c = 0.4;
        let (a, b) = table.at(c);
        let wc = -(1.0 - a).ln();
        let rate = 44_100.0_f64;
        for res in [0.3_f32, 0.65, 0.9] {
            for (kind, name) in [(0u8, "LP 12"), (1, "LP 24"), (2, "HP")] {
                for ratio in [0.3_f32, 0.6, 0.9, 1.0, 1.1, 1.5, 3.0] {
                    let w = wc * ratio;
                    let measured = 20.0 * gain_with_feedback(kind, a, res * b, w).log10();
                    let hz = (f64::from(w) * rate / std::f64::consts::TAU) as f32;
                    let shown = filter_response(c, res, kind, 0, &[hz])[0];
                    assert!(
                        (measured - shown).abs() < 1.5,
                        "{name} res {res} at {ratio}×: {measured} dB measured, {shown} dB shown"
                    );
                }
            }
        }
    }

    #[test]
    fn resonance_raises_the_peak_and_the_comb_has_teeth() {
        let hz: Vec<f32> = (0..200)
            .map(|i| 20.0 * 1000f32.powf(i as f32 / 199.0))
            .collect();
        let peak = |res| {
            filter_response(0.5, res, 0, 0, &hz)
                .into_iter()
                .fold(f32::MIN, f32::max)
        };
        assert!(peak(1.0) > peak(0.0) + 6.0, "{} {}", peak(1.0), peak(0.0));
        let comb = filter_response(0.5, 0.9, 0, EXTRA_COMB, &hz);
        let (max, min) = comb
            .iter()
            .fold((f32::MIN, f32::MAX), |(a, b), &v| (a.max(v), b.min(v)));
        assert!(comb.iter().all(|v| v.is_finite()));
        assert!(max - min > 6.0, "peaks and notches: {max} {min}");
        assert!((cutoff_of_hz(cutoff_hz(0.37)) - 0.37).abs() < 1e-4);
    }

    /// How many samples the engine's own envelope takes, at `RATE`, to leave a stage.
    fn engine_stage_seconds(
        rates: &Rates,
        from: Stage,
        start: f32,
        until: impl Fn(f32) -> bool,
    ) -> f32 {
        const RATE: f32 = 48_000.0;
        let mut env = Envelope {
            e: start,
            stage: from,
        };
        let mut n = 0;
        while !until(env.e) && n < 20_000_000 {
            env.step(rates);
            n += 1;
        }
        n as f32 / RATE
    }

    #[test]
    fn envelope_previews_take_the_times_the_engines_envelope_takes() {
        const RATE: f32 = 48_000.0;
        let mut v = DEFAULTS;
        v[3] = 0.5; // attack
        v[4] = 0.4; // decay
        v[5] = 0.5; // sustain (squared: 0.25)
        v[6] = 0.3; // release
        let preview = envelope_preview(EnvelopeKind::Level, &v, 200);
        let rates = Rates {
            attack: attack_rate(0.5, RATE),
            decay: decay_rate(0.4, RATE),
            sustain: 0.25,
            release: RATE_SCALE / (RATE * (0.3 + 0.001)),
        };
        let attack = engine_stage_seconds(&rates, Stage::Attack, 0.0, |e| e >= 1.0);
        let decay = engine_stage_seconds(&rates, Stage::Decay, 1.0, |e| {
            (e - 0.25).abs() < 0.75 * 0.005
        });
        let release = engine_stage_seconds(&rates, Stage::Release, 0.25, |e| e < 0.25 * 0.005);
        let near = |shown: f32, engine: f32| (shown - engine).abs() <= 0.03 * engine + 0.0005;
        assert!(
            near(preview.seconds[0], attack),
            "{} {attack}",
            preview.seconds[0]
        );
        assert!(
            near(preview.seconds[1], decay),
            "{} {decay}",
            preview.seconds[1]
        );
        assert!(
            near(preview.seconds[2], release),
            "{} {release}",
            preview.seconds[2]
        );
        assert!(preview.releases && preview.values.len() == 200);
        assert!(preview.values.iter().all(|x| (0.0..=1.0).contains(x)));
        assert_eq!(preview.values[0], 0.0);
        assert!((preview.sustain - 0.25).abs() < 1e-6);
        // The attack reaches 1, the decay settles at the sustain, the release ends at 0.
        let peak = preview.values.iter().cloned().fold(0.0, f32::max);
        assert!((peak - 1.0).abs() < 0.02, "{peak}");
        assert_eq!(*preview.values.last().unwrap(), 0.0);
    }

    #[test]
    fn the_filter_and_mod_envelopes_ignore_note_off_and_a_stage_of_no_time_is_vertical() {
        let mut v = DEFAULTS;
        v[26] = 0.3;
        let filter = envelope_preview(EnvelopeKind::Filter, &v, 400);
        assert!(!filter.releases);
        assert!(filter.ends[1] <= 1.0 && filter.ends[1] == filter.ends[3]);
        // An attack of 0 takes almost no width: the edge is (nearly) vertical, from 0 to 1 within a few percent.
        v[25] = 0.0;
        let zero = envelope_preview(EnvelopeKind::Filter, &v, 400);
        assert!(zero.ends[0] < 0.05, "{}", zero.ends[0]);
        let at = (zero.ends[0] * 399.0).ceil() as usize;
        assert!(zero.values[at] > 0.95 && zero.values[0] == 0.0);
        // A longer attack takes more width, a shorter one less: the picture moves with the knob.
        let mut share = |attack: f32| {
            v[25] = attack;
            envelope_preview(EnvelopeKind::Filter, &v, 100).ends[0]
        };
        let (low, mid, high) = (share(0.2), share(0.5), share(0.9));
        assert!(mid > low + 0.03, "{mid} {low}");
        assert!(high > mid + 0.05, "{high} {mid}");
        // The same for the decay and, with a release, for the release: no time, no width.
        v[26] = 0.0;
        v[25] = 0.5;
        let no_decay = envelope_preview(EnvelopeKind::Filter, &v, 100);
        assert!(no_decay.ends[1] - no_decay.ends[0] < 0.1);
        let mut level = DEFAULTS;
        level[6] = 0.0;
        let released = envelope_preview(EnvelopeKind::Level, &level, 100);
        assert!(
            released.ends[3] - released.ends[2] < 0.08,
            "{:?}",
            released.ends
        );
        // A longer decay is a longer time, on a wider share.
        v[26] = 0.3;
        let short = envelope_preview(EnvelopeKind::Filter, &v, 100);
        v[26] = 0.8;
        let long = envelope_preview(EnvelopeKind::Filter, &v, 100);
        assert!(long.seconds[1] > short.seconds[1]);
        assert!(
            long.ends[1] - long.ends[0] > short.ends[1] - short.ends[0] - 1e-6
                || long.ends[0] < short.ends[0]
        );
    }

    #[test]
    fn lfo_previews_follow_the_shapes() {
        let sine = lfo_preview(0.0, 1.0, 5);
        assert!(sine[0].abs() < 1e-6 && (sine[1] - 1.0).abs() < 1e-6);
        let saw = lfo_preview(0.5, 1.0, 5);
        assert_eq!(saw[0], -1.0);
        let square = lfo_preview(1.0, 1.0, 5);
        assert_eq!((square[0], square[4 - 1]), (-1.0, 1.0));
    }

    /// The spectrum bands the synth reports after rendering a held note with the filter's CUTOFF at `cutoff`.
    fn bands_with_cutoff(cutoff: f32) -> Vec<f32> {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(21, cutoff);
        synth.note_on(48, 1.0, 0.5);
        let (mut left, mut right) = (vec![0.0; 8192], vec![0.0; 8192]);
        for _ in 0..4 {
            synth.process(&mut left, &mut right, STOPPED);
        }
        assert_eq!(synth.analysis().len(), LIVE_LEN);
        synth.analysis()[LIVE_BANDS..].to_vec()
    }

    #[test]
    fn the_synth_reports_a_spectrum_that_follows_its_filter() {
        let (open, closed) = (bands_with_cutoff(1.0), bands_with_cutoff(0.25));
        let high = |bands: &[f32]| bands[40..].iter().sum::<f32>();
        assert!(
            bands_with_cutoff(1.0).iter().any(|v| *v > 0.3),
            "something sounds"
        );
        assert!(
            high(&open) > high(&closed) + 1.0,
            "an open filter leaves the highs: {} against {}",
            high(&open),
            high(&closed)
        );
        assert!(open.iter().chain(&closed).all(|v| (0.0..=1.0).contains(v)));
    }

    #[test]
    fn the_synth_reports_its_lfo_value_and_phase() {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(34, 1.0); // AMOUNT up
        synth.note_on(60, 1.0, 0.5);
        let (mut left, mut right) = (vec![0.0; 2000], vec![0.0; 2000]);
        let mut phases = Vec::new();
        for _ in 0..40 {
            synth.process(&mut left, &mut right, STOPPED);
            let live = synth.analysis();
            assert!((-1.0..=1.0).contains(&live[LIVE_LFO]));
            assert!((0.0..1.0).contains(&live[LIVE_PHASE]));
            phases.push(live[LIVE_PHASE]);
        }
        // Free-running while stopped: the phase moves.
        assert!(phases.windows(2).any(|w| (w[0] - w[1]).abs() > 1e-4));
    }

    /// The LFO value the synth reports after each of `blocks` blocks of `frames` frames.
    fn lfo_values(synth: &mut GmSynth, blocks: usize, frames: usize) -> Vec<f32> {
        let (mut left, mut right) = (vec![0.0; frames], vec![0.0; frames]);
        (0..blocks)
            .map(|_| {
                synth.process(&mut left, &mut right, STOPPED);
                synth.analysis()[LIVE_LFO]
            })
            .collect()
    }

    #[test]
    fn sample_and_hold_takes_a_new_value_each_cycle_inside_its_range() {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(32, 0.0); // the fastest rate: a quarter beat, 125 ms at 120 BPM
        synth.set_parameter(34, 1.0);
        synth.set_setting(LFO_MODE, 1.0);
        synth.set_setting(LFO_LOW, 0.25);
        synth.set_setting(LFO_HIGH, 0.75);
        synth.note_on(60, 1.0, 0.5);
        // 300 blocks of 1000 frames: 6 s, 48 cycles.
        let values = lfo_values(&mut synth, 300, 1000);
        assert!(
            values.iter().all(|v| (-0.5..=0.5).contains(v)),
            "{values:?}"
        );
        let mut distinct = values.clone();
        distinct.dedup();
        assert!(distinct.len() >= 20, "{} steps", distinct.len());
        // Held between steps: far fewer changes than blocks.
        assert!(distinct.len() < 100);
    }

    #[test]
    fn a_note_random_lfo_changes_at_each_note_and_holds_between_them() {
        let mut synth = GmSynth::new(RATE);
        synth.set_setting(LFO_MODE, 2.0);
        synth.set_setting(LFO_LOW, 0.0);
        synth.set_setting(LFO_HIGH, 1.0);
        let mut held = Vec::new();
        for key in [48, 52, 55, 59, 62] {
            synth.note_on(key, 1.0, 0.5);
            let during = lfo_values(&mut synth, 6, 500);
            assert!(
                during.windows(2).all(|w| w[0] == w[1]),
                "held while the note sounds"
            );
            held.push(during[0]);
        }
        let mut distinct = held.clone();
        distinct.dedup();
        assert!(distinct.len() >= 4, "{held:?}");
        assert!(held.iter().all(|v| (-1.0..=1.0).contains(v)));
    }

    #[test]
    fn the_original_lfo_shape_mode_is_untouched_by_the_random_modes() {
        let mut shape = GmSynth::new(RATE);
        let mut mode = GmSynth::new(RATE);
        mode.set_setting(LFO_MODE, 1.0);
        mode.set_setting(LFO_MODE, 0.0);
        for synth in [&mut shape, &mut mode] {
            synth.set_parameter(32, 0.0);
            synth.set_parameter(34, 1.0);
            synth.note_on(60, 1.0, 0.5);
        }
        // Mode 0 sounds as before, wherever the other modes have been.
        let a = lfo_values(&mut shape, 30, 700);
        let b = lfo_values(&mut mode, 30, 700);
        assert_eq!(a, b);
    }

    #[test]
    fn the_reported_cutoff_follows_the_envelope_and_rests_on_the_knob_when_silent() {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(21, 0.4); // CUTOFF
        synth.set_parameter(27, 1.0); // filter envelope AMOUNT up
        synth.set_parameter(26, 0.35); // DECAY
        let (mut left, mut right) = (vec![0.0; 1000], vec![0.0; 1000]);
        synth.process(&mut left, &mut right, STOPPED);
        assert!(
            (synth.analysis()[LIVE_CUTOFF] - 0.4).abs() < 1e-6,
            "rests on the knob"
        );
        synth.note_on(60, 1.0, 0.5);
        let mut seen = Vec::new();
        for _ in 0..60 {
            synth.process(&mut left, &mut right, STOPPED);
            seen.push(synth.analysis()[LIVE_CUTOFF]);
        }
        assert!(seen.iter().all(|c| (0.0..=1.0).contains(c)));
        let (first, last) = (seen[0], *seen.last().unwrap());
        assert!(first > 0.6, "the envelope opens the filter: {first}");
        assert!(
            first > last + 0.1,
            "and it closes again as it decays: {first} then {last}"
        );
    }

    /// The reported cutoff over `blocks` blocks of 1000 samples with a held note, after `setup`.
    fn cutoff_track(setup: impl FnOnce(&mut GmSynth), blocks: usize) -> Vec<f32> {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(21, 0.5); // CUTOFF
        setup(&mut synth);
        synth.note_on(60, 1.0, 0.5);
        let (mut left, mut right) = (vec![0.0; 1000], vec![0.0; 1000]);
        (0..blocks)
            .map(|_| {
                synth.process(&mut left, &mut right, STOPPED);
                synth.analysis()[LIVE_CUTOFF]
            })
            .collect()
    }

    #[test]
    fn a_route_sends_the_lfo_to_any_listed_parameter_and_amount_zero_changes_nothing() {
        let cutoff = MOD_TARGETS.iter().position(|&p| p == 21).unwrap() as f32 + 1.0;
        let route = |synth: &mut GmSynth, amount: f32| {
            synth.set_parameter(32, 0.0); // fast LFO
            synth.set_parameter(34, 0.5); // the old LFO amount stays 0
            synth.set_setting(MOD_BASE + MOD_SOURCE, 1.0);
            synth.set_setting(MOD_BASE + MOD_TARGET, cutoff);
            synth.set_setting(MOD_BASE + MOD_AMOUNT, amount);
        };
        let plain = cutoff_track(|_| {}, 40);
        let zero = cutoff_track(|s| route(s, 0.5), 40);
        assert_eq!(plain, zero, "amount 0 (the centre) is a no-op");
        let swing = |v: &[f32]| {
            v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min)
        };
        let full = cutoff_track(|s| route(s, 1.0), 40);
        assert!(
            swing(&plain) < 1e-6 && swing(&full) > 0.2,
            "{}",
            swing(&full)
        );
        // A negative amount mirrors it: the two swing around the knob in opposite senses.
        let neg = cutoff_track(|s| route(s, 0.0), 40);
        assert!(
            full.iter()
                .zip(&neg)
                .all(|(a, b)| (a + b - 1.0).abs() < 0.02
                    || *a >= 0.99
                    || *b >= 0.99
                    || *a <= 0.01
                    || *b <= 0.01)
        );
        assert!(full.iter().chain(&neg).all(|c| (0.0..=1.0).contains(c)));
    }

    #[test]
    fn an_eg_route_follows_the_envelope_and_an_off_source_does_nothing() {
        let cutoff = MOD_TARGETS.iter().position(|&p| p == 21).unwrap() as f32 + 1.0;
        let set = |synth: &mut GmSynth, source: f32| {
            synth.set_parameter(37, 0.3); // EG DECAY
            synth.set_parameter(38, 0.0); // EG SUSTAIN
            synth.set_setting(MOD_BASE + MOD_SOURCE, source);
            synth.set_setting(MOD_BASE + MOD_TARGET, cutoff);
            synth.set_setting(MOD_BASE + MOD_AMOUNT, 1.0);
        };
        let eg = cutoff_track(|s| set(s, 2.0), 60);
        assert!(
            eg[0] > eg[59] + 0.05,
            "the EG opens it, then it decays: {} {}",
            eg[0],
            eg[59]
        );
        let off = cutoff_track(|s| set(s, 0.0), 60);
        assert!(off.iter().all(|c| (c - 0.5).abs() < 1e-6));
    }

    #[test]
    fn route_settings_are_clamped_and_broken_values_are_harmless() {
        let mut synth = GmSynth::new(RATE);
        for (field, value) in [
            (MOD_SOURCE, 99.0),
            (MOD_TARGET, 1e9),
            (MOD_AMOUNT, f32::NAN),
        ] {
            synth.set_setting(MOD_BASE + 30 + field, value);
        }
        assert_eq!(synth.routes[3].amount, 0.0);
        assert_eq!(synth.routes[3].target, Some(MOD_TARGETS.len() - 1));
        synth.set_setting(MOD_BASE + 31, 0.0);
        assert_eq!(synth.routes[3].target, None, "item 0 is Off");
        synth.set_setting(MOD_BASE + 39, 1.0);
        synth.note_on(60, 1.0, 0.5);
        let (mut left, mut right) = (vec![0.0; 256], vec![0.0; 256]);
        synth.process(&mut left, &mut right, STOPPED);
        assert!(left.iter().all(|x| x.is_finite()));
    }

    /// The loudest and the quietest of a held saw's output after a note starts, with the filter DRIVE at `drive`.
    fn drive_run(drive: f32) -> (Vec<f32>, f32) {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(21, 0.6); // CUTOFF
        synth.set_parameter(22, 0.4); // RESONANCE
        synth.set_setting(FILTER_DRIVE, drive);
        synth.note_on(48, 1.0, 0.5);
        let (mut left, mut right) = (vec![0.0; 4000], vec![0.0; 4000]);
        synth.process(&mut left, &mut right, STOPPED);
        let peak = left.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
        (left, peak)
    }

    #[test]
    fn drive_zero_changes_nothing_and_more_drive_changes_the_sound_without_blowing_up() {
        let (plain, _) = drive_run(0.0);
        let (off, _) = drive_run(f32::NAN);
        assert_eq!(plain, off, "a broken value is the same as none");
        let (driven, peak) = drive_run(1.0);
        assert!(driven.iter().all(|x| x.is_finite()));
        assert_ne!(plain, driven);
        // Clipped: the waveform is flatter, so its peak is closer to its average level than the plain wave's.
        let crest = |v: &[f32]| {
            let rms = (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
            v.iter().fold(0.0_f32, |m, x| m.max(x.abs())) / rms.max(1e-6)
        };
        assert!(
            crest(&driven) < crest(&plain),
            "{} {}",
            crest(&driven),
            crest(&plain)
        );
        let (_, plain_peak) = drive_run(0.0);
        // Fuller, not much louder: the output gives back part of the gain.
        assert!(
            peak < 3.0 * plain_peak.max(1e-3),
            "{peak} against {plain_peak}"
        );
    }

    /// A held note's first 4000 samples with OSC A on menu item `item` (the position `pos`), with B silent (the default balance is A alone).
    fn table_run(item: usize, pos: f32, key: u8) -> Vec<f32> {
        let mut synth = GmSynth::new(RATE);
        prepare_setting(WAVE_A, item as f32);
        synth.set_setting(WAVE_A, item as f32);
        synth.set_parameter(POS_A, pos);
        synth.set_parameter(21, 1.0); // CUTOFF open
        synth.note_on(key, 1.0, 0.5);
        let (mut left, mut right) = (vec![0.0; 4000], vec![0.0; 4000]);
        synth.process(&mut left, &mut right, STOPPED);
        left
    }

    #[test]
    fn a_wavetable_plays_and_its_position_changes_the_sound() {
        let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        let item = TABLE_BASE; // Sine to Saw
        let (dark, bright) = (table_run(item, 0.0, 48), table_run(item, 1.0, 48));
        assert!(dark.iter().chain(&bright).all(|x| x.is_finite()));
        assert!(rms(&dark) > 0.01 && rms(&bright) > 0.01, "both play");
        assert_ne!(dark, bright, "the position changes the wave");
        // The sine end is smoother than the saw end: less change from one sample to the next.
        let rough = |v: &[f32]| v.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / rms(v);
        assert!(
            rough(&dark) < rough(&bright),
            "{} {}",
            rough(&dark),
            rough(&bright)
        );
        // A classic shape ignores the position.
        assert_eq!(
            table_run(SINE_SHAPE, 0.0, 48),
            table_run(SINE_SHAPE, 0.9, 48)
        );
    }

    const SINE_SHAPE: usize = crate::gm_waveshapes::SINE;

    #[test]
    fn every_menu_item_builds_and_a_missing_one_silences() {
        for item in 0..SOURCES {
            prepare_setting(WAVE_A, item as f32);
            assert!(cached_source(item).is_some(), "item {item}");
        }
        assert!(cached_source(SOURCES).is_none());
        assert!(table_frames(TABLE_BASE).is_some_and(|n| n > 1));
        assert!(table_frames(0).is_none(), "a classic shape is no table");
        assert_eq!(table_outline(TABLE_BASE, 3, 64).map(|v| v.len()), Some(64));
    }

    #[test]
    fn a_high_note_on_a_wavetable_stays_free_of_aliasing() {
        // A bright saw-like table at a high note: the band limit leaves no energy above Nyquist to fold back. The
        // output of a pure band-limited wave has energy only at the harmonics, so its sample-to-sample change stays
        // that of a few harmonics (no spikes).
        let out = table_run(TABLE_BASE + 1, 0.0, 100);
        assert!(out.iter().all(|x| x.is_finite()));
        let peak = out.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
        let step = out
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        assert!(step < 2.0 * peak, "no spikes: {step} against {peak}");
    }

    #[test]
    fn a_route_can_drive_the_table_position() {
        let pos = MOD_TARGETS.iter().position(|&p| p == POS_A).unwrap() as f32 + 1.0;
        let run = |amount: f32| {
            let mut synth = GmSynth::new(RATE);
            prepare_setting(WAVE_A, TABLE_BASE as f32);
            synth.set_setting(WAVE_A, TABLE_BASE as f32);
            synth.set_parameter(32, 0.0); // fast LFO
            synth.set_setting(MOD_BASE + MOD_SOURCE, 1.0);
            synth.set_setting(MOD_BASE + MOD_TARGET, pos);
            synth.set_setting(MOD_BASE + MOD_AMOUNT, amount);
            synth.note_on(48, 1.0, 0.5);
            let (mut l, mut r) = (vec![0.0; 6000], vec![0.0; 6000]);
            synth.process(&mut l, &mut r, STOPPED);
            l
        };
        assert_ne!(run(1.0), run(0.5), "a route at the centre is no modulation");
    }

    #[test]
    fn a_route_turned_off_leaves_nothing_behind() {
        let cutoff = MOD_TARGETS.iter().position(|&p| p == 21).unwrap() as f32 + 1.0;
        let run = |turn_off: &dyn Fn(&mut GmSynth)| {
            let mut synth = GmSynth::new(RATE);
            synth.set_parameter(21, 0.5);
            synth.set_parameter(32, 0.0); // fast LFO
            synth.set_setting(MOD_BASE + MOD_SOURCE, 1.0);
            synth.set_setting(MOD_BASE + MOD_TARGET, cutoff);
            synth.set_setting(MOD_BASE + MOD_AMOUNT, 1.0);
            synth.note_on(60, 1.0, 0.5);
            let (mut l, mut r) = (vec![0.0; 2000], vec![0.0; 2000]);
            for _ in 0..10 {
                synth.process(&mut l, &mut r, STOPPED);
            }
            turn_off(&mut synth);
            // The cutoff glides back for a moment, then rests on the knob.
            for _ in 0..30 {
                synth.process(&mut l, &mut r, STOPPED);
            }
            let rested: Vec<f32> = (0..20)
                .map(|_| {
                    synth.process(&mut l, &mut r, STOPPED);
                    synth.analysis()[LIVE_CUTOFF]
                })
                .collect();
            rested
        };
        let spread = |v: &[f32]| {
            v.iter().cloned().fold(f32::MIN, f32::max) - v.iter().cloned().fold(f32::MAX, f32::min)
        };
        for (name, off) in [
            (
                "source",
                &(|s: &mut GmSynth| s.set_setting(MOD_BASE + MOD_SOURCE, 0.0))
                    as &dyn Fn(&mut GmSynth),
            ),
            ("target", &|s: &mut GmSynth| {
                s.set_setting(MOD_BASE + MOD_TARGET, 0.0)
            }),
            ("amount", &|s: &mut GmSynth| {
                s.set_setting(MOD_BASE + MOD_AMOUNT, 0.5)
            }),
        ] {
            let v = run(off);
            assert!(spread(&v) < 1e-4, "{name}: still moving {v:?}");
            assert!(
                (v[0] - 0.5).abs() < 0.02,
                "{name}: rests on the knob, not {}",
                v[0]
            );
        }
    }

    #[test]
    fn turning_any_route_off_returns_the_sound_to_the_unrouted_one() {
        let run = |item: Option<usize>| {
            let mut synth = GmSynth::new(RATE);
            synth.set_parameter(32, 0.0); // fast LFO
            synth.set_parameter(15, 0.3); // MOD AMOUNT up, so FM-type targets are heard
            synth.note_on(60, 1.0, 0.5);
            let (mut l, mut r) = (vec![0.0; 2000], vec![0.0; 2000]);
            if let Some(item) = item {
                synth.set_setting(MOD_BASE + MOD_SOURCE, 1.0);
                synth.set_setting(MOD_BASE + MOD_TARGET, item as f32);
                synth.set_setting(MOD_BASE + MOD_AMOUNT, 1.0);
            }
            for _ in 0..10 {
                synth.process(&mut l, &mut r, STOPPED);
            }
            synth.set_setting(MOD_BASE + MOD_SOURCE, 0.0);
            let mut tail = Vec::new();
            for block in 0..60 {
                // The synth adds to its buffers, so each block starts from silence.
                l.fill(0.0);
                r.fill(0.0);
                synth.process(&mut l, &mut r, STOPPED);
                if block >= 40 {
                    tail.extend_from_slice(&l);
                }
            }
            tail
        };
        let plain = run(None);
        for item in 1..=MOD_TARGETS.len() {
            let routed = run(Some(item));
            let name = MOD_TARGET_NAMES[item];
            // Pitch-type targets leave the oscillator B another phase behind them while they run, and with the two
            // oscillators ring modulated that is another wave (the phase between them is the sound); the others come back
            // sample for sample.
            if matches!(MOD_TARGETS[item - 1], 11 | 12) {
                assert!(routed.iter().all(|x| x.is_finite()), "{name}");
            } else {
                let worst = (routed.iter().zip(&plain))
                    .map(|(a, b)| (a - b).abs())
                    .fold(0.0, f32::max);
                assert!(
                    worst < 1e-3,
                    "target {item} ({name}) still colours the sound by {worst}"
                );
            }
        }
    }

    #[test]
    fn a_jumping_table_position_glides_instead_of_clicking() {
        // Harmonic Sweep is smooth everywhere (no edges), so any step in the output comes from the jump itself.
        let item = TABLE_BASE + 4;
        let mut synth = GmSynth::new(RATE);
        prepare_setting(WAVE_A, item as f32);
        synth.set_setting(WAVE_A, item as f32);
        synth.set_parameter(21, 1.0); // CUTOFF open
        synth.set_parameter(POS_A, 0.0);
        synth.note_on(48, 1.0, 0.5);
        let block = |synth: &mut GmSynth| {
            let (mut l, mut r) = (vec![0.0; 2000], vec![0.0; 2000]);
            synth.process(&mut l, &mut r, STOPPED);
            l
        };
        let step = |v: &[f32]| {
            v.windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0.0, f32::max)
        };
        for _ in 0..4 {
            block(&mut synth);
        }
        let steady = step(&block(&mut synth));
        synth.set_parameter(POS_A, 1.0);
        let after = step(&block(&mut synth));
        // Unsmoothed, the wave would change by as much as the two frames differ somewhere in one sample.
        let (first, last) = (
            table_outline(item, 0, 256).unwrap(),
            table_outline(item, 31, 256).unwrap(),
        );
        // (A single oscillator is heard at half its table's level.)
        let jump = 0.5
            * first
                .iter()
                .zip(&last)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f32::max);
        assert!(
            after < 0.5 * jump && after > steady,
            "the jump {jump} is spread over a few milliseconds: step {after} after, {steady} steady"
        );
    }

    /// How many times the S&H value changes in `seconds` at 120 BPM with the SYNC item `item` and the RATE param `rate`.
    fn sample_hold_steps(item: f32, rate: f32, seconds: usize) -> usize {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(32, rate);
        synth.set_setting(LFO_MODE, 1.0);
        synth.set_setting(LFO_SYNC, item);
        synth.note_on(60, 1.0, 0.5);
        let ctx = ProcessContext {
            sample_rate: RATE,
            tempo_bpm: 120.0,
            start_beat: 0.0,
            playing: false,
        };
        let (mut left, mut right) = (vec![0.0; 100], vec![0.0; 100]);
        let mut values = Vec::new();
        for _ in 0..(RATE as usize * seconds / 100) {
            synth.process(&mut left, &mut right, ctx);
            values.push(synth.analysis()[LIVE_LFO]);
        }
        values.windows(2).filter(|w| w[0] != w[1]).count()
    }

    #[test]
    fn the_lfo_sync_menu_sets_the_cycle_in_note_values() {
        // At 120 BPM a quarter note is 0.5 s: an eighth 4 cycles a second's worth... in 2 s: 8 eighths, 16 sixteenths.
        let (eighth, sixteenth, thirty_second) = (
            sample_hold_steps(6.0, 0.9, 2),
            sample_hold_steps(3.0, 0.9, 2),
            sample_hold_steps(1.0, 0.9, 2),
        );
        assert!((7..=9).contains(&eighth), "1/8: {eighth}");
        assert!((15..=17).contains(&sixteenth), "1/16: {sixteenth}");
        assert!((31..=33).contains(&thirty_second), "1/32: {thirty_second}");
        // Orig. keeps the RATE menu: its slowest step is 32 beats, no change in 2 s.
        assert!(sample_hold_steps(0.0, 1.0, 2) <= 1);
        // A triplet is faster than the straight value: 1/8T (item 4) against 1/8 (item 6).
        assert!(sample_hold_steps(4.0, 0.9, 2) > eighth);
    }

    #[test]
    fn key_sync_restarts_the_lfo_with_each_note_and_off_lets_it_run_on() {
        let run = |keysync: f32| {
            let mut synth = GmSynth::new(RATE);
            synth.set_parameter(32, 0.5);
            synth.set_setting(LFO_KEYSYNC, keysync);
            synth.note_on(60, 1.0, 0.5);
            let (mut left, mut right) = (vec![0.0; 3000], vec![0.0; 3000]);
            synth.process(&mut left, &mut right, STOPPED);
            let before = synth.analysis()[LIVE_PHASE];
            synth.note_off(60);
            synth.note_on(64, 1.0, 0.5);
            let (mut left, mut right) = (vec![0.0; 1], vec![0.0; 1]);
            synth.process(&mut left, &mut right, STOPPED);
            (before, synth.analysis()[LIVE_PHASE])
        };
        let (before, after) = run(1.0);
        assert!(before > 0.01, "the LFO ran: {before}");
        assert!(after < 0.001, "a note restarts it: {after}");
        let (before, after) = run(0.0);
        assert!(
            after > before,
            "free-running it runs on: {before} then {after}"
        );
    }

    #[test]
    fn every_menu_slot_has_a_display_cycle_and_nothing_past_it() {
        for slot in 0..SLOTS {
            let cycle = preview_cycle(slot, 48).unwrap_or_else(|| panic!("slot {slot}"));
            assert_eq!(cycle.len(), crate::gm_waveshapes::TABLE_LEN);
            assert!(cycle.iter().all(|v| v.is_finite()), "slot {slot}");
            assert!(cycle.iter().any(|v| v.abs() > 0.1), "slot {slot} is flat");
        }
        assert!(preview_cycle(SLOTS, 48).is_none());
        // A sine has one partial: more harmonics change nothing; a saw gets sharper with them.
        let sine = |h| preview_cycle(crate::gm_waveshapes::SINE, h).unwrap();
        assert_eq!(sine(1), sine(48));
        let saw = |h| preview_cycle(crate::gm_waveshapes::SAW, h).unwrap();
        assert!(saw(48) != saw(4));
    }

    #[test]
    fn poly_takes_the_highest_free_voice_and_steals_the_oldest() {
        let mut synth = GmSynth::new(RATE);
        for key in [60, 62, 64] {
            synth.note_on(key, 1.0, 0.5);
        }
        assert_eq!(keys(&synth), [None, Some(64), Some(62), Some(60)]);
        synth.note_on(65, 1.0, 0.5);
        // All four sound: the oldest-started (60, voice 3) goes, released or not.
        synth.note_off(62);
        synth.note_on(67, 1.0, 0.5);
        assert_eq!(keys(&synth), [Some(65), Some(64), Some(62), Some(67)]);
        synth.note_on(69, 1.0, 0.5);
        assert_eq!(keys(&synth), [Some(65), Some(64), Some(69), Some(67)]);
    }

    #[test]
    fn mono_is_legato_on_voice_zero_and_falls_back_without_a_retrigger() {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(25, 0.5); // a filter env attack, to see restarts
        synth.note_on(50, 1.0, 0.5);
        synth.set_parameter(SINGLE_VOICE, 1.0);
        // Turning MONO on sent note-offs to every key.
        assert_eq!(synth.voices[3].amp.stage, Stage::Release);
        assert!(!synth.held.iter().any(|&down| down));
        synth.note_on(60, 0.5, 0.5);
        assert_eq!(keys(&synth)[0], Some(60));
        run(&mut synth, 4800);
        let before = synth.voices[0].filter_env.e;
        assert!(before > 0.1);
        // Legato: key and velocity change, nothing restarts.
        synth.note_on(64, 0.9, 0.2);
        let voice = synth.voices[0];
        assert_eq!((voice.key, voice.velocity), (64, 0.9));
        assert_eq!(voice.filter_env.e, before);
        assert_eq!(voice.pan, [1.0, 1.0], "legato keeps the pan");
        synth.note_on(55, 1.0, 0.5);
        synth.note_off(55);
        // Highest held below 55: none; lowest held above: 60.
        assert_eq!(synth.voices[0].key, 60);
        assert_eq!(synth.voices[0].filter_env.e, before);
        synth.note_off(60);
        assert_eq!(synth.voices[0].key, 64);
        synth.note_off(64);
        assert_eq!(synth.voices[0].amp.stage, Stage::Release);
        // Released: the next note retriggers.
        synth.note_on(62, 1.0, 0.5);
        assert_eq!(synth.voices[0].filter_env.e, 0.0);
        assert_eq!(synth.voices[0].amp.stage, Stage::Attack);
    }

    #[test]
    fn glide_starts_at_the_previous_note_and_closes_one_pole() {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(29, 0.5);
        synth.note_on(48, 1.0, 0.5);
        synth.note_off(48);
        synth.note_on(60, 1.0, 0.5);
        let voice = |synth: &GmSynth| synth.voices[2];
        assert_eq!(voice(&synth).p, 48.0);
        // p += (target − p)·32/(0.1·sr·v29 + 1) per block.
        let k = BLOCK as f32 / (0.1 * RATE * 0.5 + 1.0);
        run(&mut synth, 10 * BLOCK);
        let expected = 60.0 - 12.0 * (1.0 - k).powi(10);
        assert!(
            (voice(&synth).p - expected).abs() < 1e-3,
            "{}",
            voice(&synth).p
        );
        // AUTO: no glide unless another key is held.
        let mut auto = GmSynth::new(RATE);
        auto.set_parameter(29, 0.5);
        auto.set_parameter(31, 0.001); // on at any nonzero v
        auto.note_on(48, 1.0, 0.5);
        auto.note_off(48);
        auto.note_on(60, 1.0, 0.5);
        assert_eq!(auto.voices[2].p, 60.0);
        auto.note_on(67, 1.0, 0.5);
        assert_eq!(auto.voices[1].p, 60.0);
        // Slide 0 snaps at the first block.
        let mut snap = GmSynth::new(RATE);
        snap.note_on(48, 1.0, 0.5);
        snap.note_on(60, 1.0, 0.5);
        run(&mut snap, 1);
        assert_eq!(snap.voices[2].p, 60.0);
    }

    #[test]
    fn a_voice_idles_once_its_amp_env_aims_at_zero_below_1e_4() {
        // Idle when the amp env is < 10⁻⁴ with target 0, a decay to sustain 0 included (no note-off).
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(4, 0.0);
        synth.set_parameter(5, 0.0);
        synth.note_on(60, 1.0, 0.5);
        assert_eq!(keys(&synth), [None, None, None, Some(60)]);
        run(&mut synth, 480);
        assert_eq!(keys(&synth), [None; VOICES]);
        synth.note_on(62, 1.0, 0.5);
        assert_eq!(keys(&synth)[3], Some(62));
        // Sustain above 0 keeps it.
        synth.set_parameter(5, 0.5);
        synth.note_on(64, 1.0, 0.5);
        run(&mut synth, 480);
        assert_eq!(keys(&synth)[2], Some(64));
    }

    #[test]
    fn semitone_steps_follow_the_round_12_laws() {
        // At v = f32(k/48). TRNSP: trunc(fma_f32(v, 48, −24)), the DW table. PITCH B audio: trunc(f64(v)·48 − 24),
        // the Slicer table.
        let transpose_off = [
            (16, -7),
            (19, -4),
            (22, -1),
            (25, 0),
            (28, 3),
            (31, 6),
            (34, 9),
            (37, 12),
            (40, 15),
        ];
        for k in 0..=48_i32 {
            let v = (f64::from(k) / 48.0) as f32;
            let transpose = transpose_off
                .iter()
                .find(|o| o.0 == k)
                .map_or(k - 24, |o| o.1);
            assert_eq!(transpose_step(v), transpose as f32, "TRNSP k {k}");
            let pitch_b = match k {
                1 | 2 | 4 | 8 | 11 | 13 | 16 | 19 | 22 => k - 23,
                25.. if (k - 25) % 3 == 0 => k - 25,
                _ => k - 24,
            };
            let mut params = DEFAULTS;
            params[11] = v;
            let expected = (f64::from(pitch_b) / 12.0).exp2() as f32;
            assert_eq!(ratio_b(&params), expected, "PITCH B k {k}");
        }
    }

    #[test]
    fn a_decay_to_zero_ends_at_zero() {
        let mut env = Envelope {
            e: 1.0,
            stage: Stage::Decay,
        };
        let rates = Rates {
            decay: decay_rate(0.0, RATE),
            ..Rates::default()
        };
        // k = 0.18: 1e-20 after 226 samples, while plain f32 would only underflow after about 500.
        let level = (0..300).map(|_| env.step(&rates)).last().unwrap();
        assert_eq!(level, 0.0);
    }

    #[test]
    fn b_locks_to_a_at_retrigger() {
        let mut synth = GmSynth::new(RATE);
        synth.set_parameter(14, 0.25);
        synth.set_parameter(11, 0.75); // +12 st: f_B/f_A = 2
        synth.note_on(60, 1.0, 0.5);
        let voice = synth.voices[3];
        for (a, b) in voice.phase_a.iter().zip(voice.phase_b) {
            let expected = (0.25 + f64::from(*a) / 4_294_967_296.0 * 2.0).fract();
            let found = f64::from(b) / 4_294_967_296.0;
            assert!((found - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn fx_slots_read_their_params_by_the_round_8_laws() {
        use crate::multifx::{DELAY, FLANGER, LPF, VOX};
        let kinds = |v: &[f32; PARAMS]| [0, 1, 2].map(|slot| fx_slot(v, slot).0);
        // The defaults 1/9, 5/9 and 3/9 (exact), all off. Enable is on at any v ≠ 0:
        // 0.001 and 0.5 included.
        let mut v = DEFAULTS;
        assert_eq!(kinds(&v), [None; 3]);
        for on in [0.001, 0.5, 1.0] {
            (v[47], v[54], v[61]) = (on, on, on);
            assert_eq!(
                kinds(&v),
                [Some(FLANGER), Some(DELAY), Some(LPF)],
                "Enable {on}"
            );
        }
        // kind = trunc(9v), plain: k/9 is step k, and just below it is step k − 1.
        for k in 1..=9 {
            v[53] = k as f32 / 9.0;
            assert_eq!(fx_slot(&v, 1).0, Some(k), "kind {k}/9");
            v[53] = k as f32 / 9.0 - 1e-4;
            assert_eq!(fx_slot(&v, 1).0, Some(k - 1), "kind {k}/9 − 1e-4");
        }
        (v[46], v[53], v[60]) = (1.0, 0.99, 0.0);
        assert_eq!(kinds(&v), [Some(VOX), Some(8), Some(0)]);
        // Sync R by trunc(9v), shape trunc(2v) (≥ 2 Square), amount sign(2v − 1)·(2v − 1)², X and Y raw.
        for (sync, rate, shape_v, shape, amount_v, amount) in [
            (0.0, 0.25, 0.0, 0, 0.5, 0.0),
            (1.0 / 9.0, 0.375, 0.49, 0, 0.25, -0.25),
            (5.0 / 9.0, 2.0, 0.5, 1, 0.75, 0.25),
            (0.999, 16.0, 0.99, 1, 0.0, -1.0),
            (4.0 / 9.0 - 1e-4, 0.75, 0.4999, 0, 0.5, 0.0),
            (4.0 / 9.0, 1.0, 0.5, 1, 0.5, 0.0),
            (1.0, 32.0, 1.0, 2, 0.9, 0.64),
        ] {
            (v[55], v[56], v[57], v[58], v[59]) = (sync, shape_v, amount_v, 0.3, 0.8);
            let (_, settings) = fx_slot(&v, 1);
            let expected = Settings {
                x: 0.3,
                y: 0.8,
                rate_beats: rate,
                amount,
                shape,
            };
            assert!(
                (settings.amount - amount).abs() < 1e-6,
                "{settings:?} != {expected:?}"
            );
            assert_eq!(Settings { amount, ..settings }, expected);
        }
    }
}
