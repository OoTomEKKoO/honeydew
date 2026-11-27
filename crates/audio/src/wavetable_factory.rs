//! The built-in wavetables (Honeydew's own designs, made from maths, with no sampled content): each is a row of
//! [`FRAMES`] single-cycle frames that [`crate::wavetable::FrameTable::from_cycles`] turns into band-limited tables.
//! Their names only say what the morph is drawn to evoke.

use std::f64::consts::{PI, TAU};

/// Frames in every built-in table.
pub const FRAMES: usize = 32;
/// Points in every generated cycle (the table resamples them to its own frame length).
const POINTS: usize = 1024;
/// The partials the additive recipes use (well below the 512 that fit in [`POINTS`]).
const PARTIALS: usize = 200;

/// The menu names, in order.
pub const NAMES: [&str; 12] = [
    "Sine to Saw",
    "Saw to Square",
    "Pulse Width",
    "Vowels",
    "Harmonic Sweep",
    "FM Sweep",
    "Hard Sync",
    "Organ Bars",
    "Bit Crush",
    "Phase Scramble",
    "Hollow Bell",
    "Reed to Brass",
];

/// A cycle from partial amplitudes `amp(h)` (h = 1..) and phases `phase(h)` in radians.
fn additive(amp: impl Fn(usize) -> f64, phase: impl Fn(usize) -> f64) -> Vec<f32> {
    let mut cycle = vec![0.0_f64; POINTS];
    for h in 1..=PARTIALS {
        let (a, p) = (amp(h), phase(h));
        if a.abs() < 1e-9 {
            continue;
        }
        for (n, v) in cycle.iter_mut().enumerate() {
            *v += a * (TAU * (h * n) as f64 / POINTS as f64 + p).sin();
        }
    }
    cycle.into_iter().map(|v| v as f32).collect()
}

/// A cycle from a function of the phase in 0..1.
fn shape(f: impl Fn(f64) -> f64) -> Vec<f32> {
    (0..POINTS)
        .map(|n| f(n as f64 / POINTS as f64) as f32)
        .collect()
}

/// Frame `i`'s place in the row, 0…1.
fn t(i: usize) -> f64 {
    i as f64 / (FRAMES - 1) as f64
}

/// A deterministic pseudo-random number in 0..1 for `n` (so a table is the same on every run).
fn hash(n: u64) -> f64 {
    let mut x = n.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    (x >> 11) as f64 / (1_u64 << 53) as f64
}

/// The frames of built-in table `index`, or `None` past [`NAMES`].
pub fn cycles(index: usize) -> Option<Vec<Vec<f32>>> {
    let frame: Box<dyn Fn(usize) -> Vec<f32>> = match index {
        // The harmonics of a saw open one by one: a sine that grows brighter and brighter.
        0 => Box::new(|i| {
            let top = 1.0 + t(i) * (PARTIALS as f64 - 1.0);
            additive(
                |h| {
                    if (h as f64) <= top {
                        1.0 / h as f64
                    } else {
                        0.0
                    }
                },
                |_| 0.0,
            )
        }),
        // Even harmonics fade out: a saw becomes a square.
        1 => Box::new(|i| {
            additive(
                |h| (1.0 / h as f64) * if h % 2 == 0 { 1.0 - t(i) } else { 1.0 },
                |_| 0.0,
            )
        }),
        // A pulse narrows from half to a twentieth of the cycle.
        2 => Box::new(|i| {
            let w = 0.5 - 0.45 * t(i);
            additive(
                |h| (PI * h as f64 * w).sin() / (PI * h as f64),
                |h| PI * h as f64 * w - PI / 2.0,
            )
        }),
        // A saw through two formants that move from one vowel to the next: a, e, i, o, u.
        3 => Box::new(|i| {
            const VOWELS: [(f64, f64); 5] = [
                (8.0, 12.0),
                (5.0, 22.0),
                (3.0, 26.0),
                (5.0, 9.0),
                (3.0, 8.0),
            ];
            let x = t(i) * (VOWELS.len() - 1) as f64;
            let (k, f) = ((x.floor() as usize).min(VOWELS.len() - 2), x.fract());
            let (a, b) = (VOWELS[k], VOWELS[k + 1]);
            let (f1, f2) = (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
            let bump = |h: f64, c: f64, w: f64| (-((h - c) / w).powi(2)).exp();
            additive(
                |h| {
                    let h = h as f64;
                    (1.0 / h.sqrt()) * (bump(h, f1, 1.6) + 0.7 * bump(h, f2, 2.4)) + 0.02 / h
                },
                |_| 0.0,
            )
        }),
        // One bright partial climbs the harmonic series over the fundamental.
        4 => Box::new(|i| {
            let at = 1.0 + t(i) * 31.0;
            additive(
                |h| {
                    let h = h as f64;
                    (if h == 1.0 { 1.0 } else { 0.0 }) + 0.8 * (-((h - at) / 0.8).powi(2)).exp()
                },
                |_| 0.0,
            )
        }),
        // Phase modulation with a growing index: sin(x + I·sin(2x)).
        5 => Box::new(|i| {
            let index = 6.0 * t(i);
            shape(|p| (TAU * p + index * (2.0 * TAU * p).sin()).sin())
        }),
        // A sawtooth read faster and faster and cut at the cycle's end, as in hard sync.
        6 => Box::new(|i| {
            let ratio = 1.0 + 3.5 * t(i);
            shape(|p| 2.0 * (ratio * p).fract() - 1.0)
        }),
        // Drawbars: the first nine partials of an organ, brought in from the lowest to the highest.
        7 => Box::new(|i| {
            additive(
                |h| {
                    if h > 9 {
                        return 0.0;
                    }
                    let open = (t(i) * 9.0 - (h as f64 - 1.0)).clamp(0.0, 1.0);
                    (open + if h == 1 { 1.0 } else { 0.0 }).min(1.0) / (h as f64).sqrt()
                },
                |_| 0.0,
            )
        }),
        // A saw with fewer and fewer levels.
        8 => Box::new(|i| {
            let levels = (64.0 * (1.0 - t(i)) + 3.0 * t(i)).round().max(2.0);
            shape(|p| ((2.0 * p - 1.0) * levels / 2.0).round() * 2.0 / levels)
        }),
        // The same loud partials with phases that drift from ordered to scrambled: the look changes, and the sound
        // turns from a saw to something rougher.
        9 => Box::new(|i| {
            additive(
                |h| 1.0 / (h as f64).powf(0.9),
                |h| t(i) * TAU * hash(h as u64),
            )
        }),
        // Partials at the ratios of a struck bell, thinning out as the frames go on.
        10 => Box::new(|i| {
            const BELL: [usize; 8] = [1, 2, 3, 4, 5, 7, 9, 12];
            additive(
                |h| {
                    BELL.iter().position(|&b| b == h).map_or(0.0, |k| {
                        (1.0 - t(i) * k as f64 / 8.0).max(0.0) / (1.0 + k as f64 * 0.4)
                    })
                },
                |h| h as f64,
            )
        }),
        // A thin odd-harmonic reed that fills with even harmonics and brightens into brass.
        11 => Box::new(|i| {
            additive(
                |h| {
                    let h = h as f64;
                    let tilt = 1.0 / h.powf(1.6 - 0.9 * t(i));
                    if (h as usize).is_multiple_of(2) {
                        tilt * t(i)
                    } else {
                        tilt
                    }
                },
                |_| 0.0,
            )
        }),
        _ => return None,
    };
    Some((0..FRAMES).map(frame).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wavetable::FrameTable;

    #[test]
    fn every_built_in_table_builds_into_loud_distinct_frames() {
        for (index, name) in NAMES.iter().enumerate() {
            let cycles = cycles(index).unwrap_or_else(|| panic!("{name} has no recipe"));
            assert_eq!(cycles.len(), FRAMES, "{name}");
            assert!(cycles.iter().flatten().all(|v| v.is_finite()), "{name}");
            let table = FrameTable::from_cycles(&cycles).expect(name);
            let (first, last) = (table.outline(0, 256), table.outline(FRAMES - 1, 256));
            let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
            assert!(
                (rms(&first) - crate::wavetable::FRAME_RMS).abs() < 0.02,
                "{name}: {}",
                rms(&first)
            );
            assert!(
                (rms(&last) - crate::wavetable::FRAME_RMS).abs() < 0.02,
                "{name}: {}",
                rms(&last)
            );
            let apart = first
                .iter()
                .zip(&last)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f32::max);
            assert!(
                apart > 0.05,
                "{name}: first and last frame are the same wave ({apart})"
            );
            // Neighbouring frames are close: a morph, not a jump. Measured as the RMS of their difference, since an edge
            // that moves (a pulse) differs by its whole height at a few points however small the step is.
            let step = (0..FRAMES - 1)
                .map(|i| {
                    let (a, b) = (table.outline(i, 256), table.outline(i + 1, 256));
                    let diff: Vec<f32> = a.iter().zip(&b).map(|(a, b)| a - b).collect();
                    rms(&diff)
                })
                .fold(0.0, f32::max);
            assert!(
                step < 0.6,
                "{name}: neighbouring frames differ by {step} RMS"
            );
        }
        assert!(cycles(NAMES.len()).is_none());
    }

    #[test]
    fn the_tables_are_the_same_on_every_run() {
        for (index, name) in NAMES.iter().enumerate() {
            assert_eq!(cycles(index), cycles(index), "{name}");
        }
    }
}
