//! Wavetables for the GM Synth's oscillators (Honeydew's own): a table is a row of single-cycle frames, and a position
//! between 0 and 1 morphs from the first frame to the last, as in Serum or Vital.
//!
//! - **Band limits.** Every frame is stored in [`LEVELS`] versions, each with half the harmonics of the one before
//!   (1024, 512, … 1), made once on the app thread by an FFT. A voice reads the finest version whose top harmonic stays
//!   below Nyquist at its pitch ([`FrameTable::level_for`]), so a high note never aliases and a low one keeps its
//!   brightness.
//! - **Reading.** Linear between the 2048 points of a frame and linear between two frames by the position: a morph is
//!   a smooth blend, never a step ([`FramePair::read`]). The reads allocate nothing.
//! - **Loudness.** Frames are scaled to one RMS ([`FRAME_RMS`]), so a morph from a sine to a saw does not jump in
//!   level; a frame that is silent stays silent.
//! - **Sources.** [`FrameTable::from_cycles`] takes any cycles of any length (a WAV cut into frames, a factory
//!   recipe), resamples each to [`FRAME_LEN`] and builds the versions. The 29 classic shapes keep their own tables
//!   (`gm_waveshapes`) and are not read through here.

use std::f64::consts::TAU;

/// Points per frame.
pub const FRAME_LEN: usize = 2048;
/// The most frames a table holds (a longer source is cut).
pub const MAX_FRAMES: usize = 256;
/// Band-limited versions per frame: the version `k` holds harmonics 1..=(1024 >> k).
pub const LEVELS: usize = 11;
/// The RMS every frame is scaled to (between the engine's sine and its square).
pub const FRAME_RMS: f32 = 0.6;
/// The most a quiet frame is amplified by the loudness law.
const MAX_GAIN: f64 = 16.0;
/// One frame version's stored length: the points and one more, a copy of the first, so a read needs no wrap.
const STRIDE: usize = FRAME_LEN + 1;
/// The phase bits above the frame index, and the fraction below it.
const INDEX_SHIFT: u32 = 32 - FRAME_LEN.trailing_zeros();
const FRACTION: f32 = 1.0 / (1_u64 << INDEX_SHIFT) as f32;

/// A wavetable: its frames in all their band-limited versions.
pub struct FrameTable {
    frames: usize,
    /// `frames × LEVELS × STRIDE` points, frame major.
    data: Box<[f32]>,
}

/// Two neighbouring frames of one band-limited version, and how far the position has moved from the first to the
/// second: the pair a voice reads for a whole control block.
#[derive(Clone, Copy)]
pub struct FramePair<'a> {
    a: &'a [f32],
    b: &'a [f32],
    mix: f32,
}

impl FramePair<'_> {
    /// One sample at the 32-bit `phase` (a full turn is 2³²), linear between points and between the frames.
    pub fn read(&self, phase: u32) -> f32 {
        let i = (phase >> INDEX_SHIFT) as usize;
        let frac = (phase & ((1 << INDEX_SHIFT) - 1)) as f32 * FRACTION;
        let lerp = |t: &[f32]| t[i] + (t[i + 1] - t[i]) * frac;
        let a = lerp(self.a);
        if self.mix == 0.0 {
            a
        } else {
            a + (lerp(self.b) - a) * self.mix
        }
    }
}

impl FrameTable {
    /// A table of `cycles` (the first [`MAX_FRAMES`]), each resampled to [`FRAME_LEN`] points. `None` for no cycle, or
    /// for a cycle shorter than two points or with a value that is not a number.
    pub fn from_cycles(cycles: &[Vec<f32>]) -> Option<Self> {
        let cycles = &cycles[..cycles.len().min(MAX_FRAMES)];
        if cycles.is_empty()
            || cycles
                .iter()
                .any(|c| c.len() < 2 || c.iter().any(|v| !v.is_finite()))
        {
            return None;
        }
        let mut data = vec![0.0_f32; cycles.len() * LEVELS * STRIDE].into_boxed_slice();
        for (n, cycle) in cycles.iter().enumerate() {
            let frame = resample(cycle);
            let (re, im) = spectrum(&frame);
            let gain = loudness_gain(&re, &im);
            for level in 0..LEVELS {
                let table = band_limited(&re, &im, (FRAME_LEN / 2) >> level, gain);
                let at = (n * LEVELS + level) * STRIDE;
                data[at..at + FRAME_LEN].copy_from_slice(&table);
                data[at + FRAME_LEN] = table[0];
            }
        }
        Some(Self {
            frames: cycles.len(),
            data,
        })
    }

    /// How many frames the table has (at least 1).
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// The band-limited version a voice at `freq` Hz reads at `sample_rate`: the finest whose top harmonic is at most
    /// Nyquist (harmonic h·f < sr/2), at least the last (the fundamental alone). A broken input reads the last.
    pub fn level_for(freq: f32, sample_rate: f32) -> usize {
        let (f, sr) = (f64::from(freq).abs(), f64::from(sample_rate));
        if !f.is_finite() || !sr.is_finite() || sr <= 0.0 || f <= 0.0 {
            return if f == 0.0 && sr > 0.0 { 0 } else { LEVELS - 1 };
        }
        let allowed = (sr / (2.0 * f)).floor();
        (0..LEVELS)
            .find(|&k| ((FRAME_LEN / 2) >> k) as f64 <= allowed)
            .unwrap_or(LEVELS - 1)
    }

    fn frame(&self, frame: usize, level: usize) -> &[f32] {
        let at = (frame.min(self.frames - 1) * LEVELS + level.min(LEVELS - 1)) * STRIDE;
        &self.data[at..at + STRIDE]
    }

    /// The pair of frames at `position` (0…1, clamped; a position that is not a number is 0) in `level`.
    pub fn pair(&self, level: usize, position: f32) -> FramePair<'_> {
        let p = if position.is_finite() {
            position.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let x = p * (self.frames - 1) as f32;
        let first = (x.floor() as usize).min(self.frames - 1);
        FramePair {
            a: self.frame(first, level),
            b: self.frame(first + 1, level),
            mix: x - first as f32,
        }
    }

    /// Frame `frame`'s full-bandwidth shape as `points` values over one cycle, for the pictures (the fine version, read
    /// at evenly spaced phases).
    pub fn outline(&self, frame: usize, points: usize) -> Vec<f32> {
        let table = self.frame(frame, 0);
        (0..points)
            .map(|i| {
                let phase = (i as u64 * (1 << 32) / points.max(1) as u64) as u32;
                let (j, frac) = (
                    (phase >> INDEX_SHIFT) as usize,
                    (phase & ((1 << INDEX_SHIFT) - 1)) as f32 * FRACTION,
                );
                table[j] + (table[j + 1] - table[j]) * frac
            })
            .collect()
    }
}

/// The frame lengths a file is tried in, longest first ([`cycles_from_samples`]).
pub const FRAME_LENGTHS: [usize; 4] = [2048, 1024, 512, 256];
/// The longest file read as one single cycle when its length fits no frame length.
const SINGLE_CYCLE_MAX: usize = 4096;

/// The cycles of a wavetable file's `samples` (a mono mix), as frames of `frame_len` points, or of the length the file's
/// size suggests (`None`: the longest of [`FRAME_LENGTHS`] that divides it, as Serum and Vital tables are 2048 a frame;
/// a file that fits none and is at most [`SINGLE_CYCLE_MAX`] points long is one single cycle; a longer one is cut into
/// frames of 2048 and the rest dropped). `None` for a file shorter than two points or with a value that is not a number.
pub fn cycles_from_samples(samples: &[f32], frame_len: Option<usize>) -> Option<Vec<Vec<f32>>> {
    if samples.len() < 2 || samples.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let len = match frame_len {
        Some(n) => n.clamp(2, samples.len()),
        None => FRAME_LENGTHS
            .into_iter()
            .find(|&n| samples.len() >= n && samples.len() % n == 0)
            .unwrap_or(if samples.len() <= SINGLE_CYCLE_MAX {
                samples.len()
            } else {
                FRAME_LEN
            }),
    };
    let frames: Vec<Vec<f32>> = samples
        .chunks_exact(len)
        .take(MAX_FRAMES)
        .map(<[f32]>::to_vec)
        .collect();
    (!frames.is_empty()).then_some(frames)
}

/// `cycles` as 16-bit values, one frame after another (the song keeps a file's table this way): full scale is ±1, a value
/// past it is clipped.
pub fn quantize(cycles: &[Vec<f32>]) -> Vec<i16> {
    cycles
        .iter()
        .flatten()
        .map(|&v| (v.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16)
        .collect()
}

/// [`quantize`]'s data as cycles of `frame_len` points (a rest shorter than a frame is dropped).
pub fn dequantize(data: &[i16], frame_len: usize) -> Vec<Vec<f32>> {
    data.chunks_exact(frame_len.max(2))
        .take(MAX_FRAMES)
        .map(|c| {
            c.iter()
                .map(|&v| f32::from(v) / f32::from(i16::MAX))
                .collect()
        })
        .collect()
}

/// `cycle` as [`FRAME_LEN`] points: the cycle read at evenly spaced phases, linear and wrapping. DC is not removed
/// here (the spectrum drops it).
fn resample(cycle: &[f32]) -> Vec<f64> {
    let n = cycle.len();
    (0..FRAME_LEN)
        .map(|i| {
            let x = i as f64 * n as f64 / FRAME_LEN as f64;
            let (j, frac) = (x.floor() as usize, x.fract());
            let (a, b) = (f64::from(cycle[j % n]), f64::from(cycle[(j + 1) % n]));
            a + (b - a) * frac
        })
        .collect()
}

/// The spectrum of a frame: `(re, im)` of bins 0..=1024, with the DC bin dropped.
fn spectrum(frame: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut re = frame.to_vec();
    let mut im = vec![0.0; FRAME_LEN];
    fft(&mut re, &mut im, false);
    re.truncate(FRAME_LEN / 2 + 1);
    im.truncate(FRAME_LEN / 2 + 1);
    re[0] = 0.0;
    im[0] = 0.0;
    (re, im)
}

/// The gain that scales a frame to [`FRAME_RMS`] (Parseval on the kept bins), at most [`MAX_GAIN`]; 0 for silence.
fn loudness_gain(re: &[f64], im: &[f64]) -> f64 {
    // The frame is Σ over bins 1..=1024 of a cosine of amplitude 2|X|/N, whose power is half its square.
    let n = FRAME_LEN as f64;
    let power: f64 = (1..re.len())
        .map(|k| {
            let amp = 2.0 * (re[k] * re[k] + im[k] * im[k]).sqrt() / n;
            0.5 * amp * amp
        })
        .sum();
    let rms = power.sqrt();
    if rms < 1e-9 {
        0.0
    } else {
        (f64::from(FRAME_RMS) / rms).min(MAX_GAIN)
    }
}

/// The frame with only harmonics 1..=`top`, times `gain`, as [`FRAME_LEN`] points.
fn band_limited(re: &[f64], im: &[f64], top: usize, gain: f64) -> Vec<f32> {
    let mut r = vec![0.0; FRAME_LEN];
    let mut i = vec![0.0; FRAME_LEN];
    for k in 1..=top.min(FRAME_LEN / 2) {
        // The mirrored half makes the inverse transform real.
        r[k] = re[k];
        i[k] = im[k];
        if k < FRAME_LEN / 2 {
            r[FRAME_LEN - k] = re[k];
            i[FRAME_LEN - k] = -im[k];
        }
    }
    fft(&mut r, &mut i, true);
    let scale = gain / FRAME_LEN as f64;
    r.iter().map(|&v| (v * scale) as f32).collect()
}

/// An in-place radix-2 FFT of `re` + i·`im` (a power of two long); `inverse` without the 1/N scale.
fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    debug_assert!(n.is_power_of_two() && im.len() == n);
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let sign = if inverse { 1.0 } else { -1.0 };
    let mut len = 2;
    while len <= n {
        let step = sign * TAU / len as f64;
        let (wr, wi) = (step.cos(), step.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * cr - im[b] * ci, re[b] * ci + im[b] * cr);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                (cr, ci) = (cr * wr - ci * wi, cr * wi + ci * wr);
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| (TAU * i as f64 / len as f64).sin() as f32)
            .collect()
    }

    fn saw(len: usize) -> Vec<f32> {
        (0..len)
            .map(|i| 2.0 * i as f32 / len as f32 - 1.0)
            .collect()
    }

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt()
    }

    /// The magnitudes of harmonics 1.. of a stored frame version.
    fn harmonics(table: &FrameTable, frame: usize, level: usize) -> Vec<f64> {
        let mut re: Vec<f64> = table.frame(frame, level)[..FRAME_LEN]
            .iter()
            .map(|&v| f64::from(v))
            .collect();
        let mut im = vec![0.0; FRAME_LEN];
        fft(&mut re, &mut im, false);
        (1..=FRAME_LEN / 2)
            .map(|k| 2.0 * (re[k] * re[k] + im[k] * im[k]).sqrt() / FRAME_LEN as f64)
            .collect()
    }

    #[test]
    fn a_sine_frame_survives_every_band_limit_and_every_frame_has_the_same_rms() {
        let table = FrameTable::from_cycles(&[sine(600), saw(2048)]).unwrap();
        assert_eq!(table.frames(), 2);
        for frame in 0..2 {
            let fine = table.outline(frame, 2048);
            assert!(
                (rms(&fine) - FRAME_RMS).abs() < 0.01,
                "frame {frame}: {}",
                rms(&fine)
            );
        }
        // The sine has one harmonic, so even the version with only the fundamental is the same wave.
        let first = table.frame(0, 0)[..FRAME_LEN].to_vec();
        let last = table.frame(0, LEVELS - 1)[..FRAME_LEN].to_vec();
        let worst = first
            .iter()
            .zip(&last)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max);
        assert!(worst < 1e-4, "{worst}");
    }

    #[test]
    fn each_version_holds_no_harmonic_above_its_limit() {
        let table = FrameTable::from_cycles(&[saw(2048)]).unwrap();
        for level in 0..LEVELS {
            let limit = (FRAME_LEN / 2) >> level;
            let h = harmonics(&table, 0, level);
            let above = h[limit..].iter().cloned().fold(0.0, f64::max);
            let below = h[..limit].iter().cloned().fold(0.0, f64::max);
            assert!(
                above < 1e-4 * below.max(1e-3),
                "level {level}: {above} above {limit}"
            );
        }
        // And a saw keeps its harmonics 1/h below the limit: the fundamental is the loudest.
        let h = harmonics(&table, 0, LEVELS - 1);
        assert!(h[0] > 0.5 && h[1..].iter().all(|v| *v < 1e-4));
    }

    #[test]
    fn the_level_follows_the_pitch_and_never_lets_a_harmonic_pass_nyquist() {
        let sr = 48_000.0;
        let mut last = 0;
        for f in [20.0, 50.0, 110.0, 440.0, 1000.0, 3000.0, 8000.0, 20_000.0] {
            let level = FrameTable::level_for(f, sr);
            assert!(level >= last, "coarser with pitch: {f}");
            last = level;
            let top = ((FRAME_LEN / 2) >> level) as f32;
            assert!(
                top * f <= sr / 2.0 + f || level == LEVELS - 1,
                "{f} Hz level {level}"
            );
        }
        assert_eq!(
            FrameTable::level_for(20.0, sr),
            0,
            "a low note keeps all its harmonics"
        );
        assert_eq!(FrameTable::level_for(f32::NAN, sr), LEVELS - 1);
        assert_eq!(FrameTable::level_for(440.0, 0.0), LEVELS - 1);
    }

    #[test]
    fn the_position_blends_neighbouring_frames_smoothly() {
        let table = FrameTable::from_cycles(&[sine(2048), saw(2048), sine(2048)]).unwrap();
        let phase = 0x3000_0000_u32;
        let at = |p: f32| table.pair(0, p).read(phase);
        // On a frame the table reads that frame; halfway it is the mean of the two neighbours.
        let (a, b) = (at(0.0), at(0.5));
        assert!((at(0.25) - 0.5 * (a + b)).abs() < 1e-5);
        assert!((at(1.0) - a).abs() < 1e-5, "the last frame is a sine again");
        // A sweep of the position moves the sample in small steps (no jumps).
        let sweep: Vec<f32> = (0..=200).map(|i| at(i as f32 / 200.0)).collect();
        let jump = sweep
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        assert!(jump < 0.05, "{jump}");
        // A broken position is the first frame.
        assert_eq!(at(f32::NAN), a);
        assert_eq!(at(-3.0), a);
    }

    #[test]
    fn reading_interpolates_between_points_and_wraps_without_a_click() {
        let table = FrameTable::from_cycles(&[sine(2048)]).unwrap();
        let pair = table.pair(0, 0.0);
        // A phase ramp over a whole cycle gives a smooth sine: no step above the largest slope of the wave.
        let samples: Vec<f32> = (0..4096u64)
            .map(|i| pair.read((i * (1 << 32) / 4096) as u32))
            .collect();
        let step = samples
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0, f32::max);
        assert!(step < FRAME_RMS * 1.5 * 0.0031 * 2.0, "{step}");
        // The seam: the last phase before the turn and the first after it are neighbours.
        assert!((pair.read(u32::MAX) - pair.read(0)).abs() < 1e-3);
    }

    #[test]
    fn a_files_frames_follow_its_length_or_its_given_frame_size() {
        let wave = |n: usize| {
            (0..n)
                .map(|i| (i as f32 / n as f32).sin())
                .collect::<Vec<f32>>()
        };
        // 4 × 2048: four frames of 2048, as Serum tables are.
        let four = cycles_from_samples(&wave(4 * 2048), None).unwrap();
        assert_eq!((four.len(), four[0].len()), (4, 2048));
        // 3 × 1024 divides by 1024 only: three frames of 1024. 512 alone: one frame of 512.
        assert_eq!(cycles_from_samples(&wave(3 * 1024), None).unwrap().len(), 3);
        let one = cycles_from_samples(&wave(512), None).unwrap();
        assert_eq!((one.len(), one[0].len()), (1, 512));
        // A length that fits no frame size is one single cycle when short, and cut into 2048s when long.
        let odd = cycles_from_samples(&wave(1000), None).unwrap();
        assert_eq!((odd.len(), odd[0].len()), (1, 1000));
        let long = cycles_from_samples(&wave(10_000), None).unwrap();
        assert_eq!((long.len(), long[0].len()), (4, 2048));
        // A given size cuts the file by it (the rest dropped), and never past the file.
        assert_eq!(
            cycles_from_samples(&wave(2500), Some(500)).unwrap().len(),
            5
        );
        assert_eq!(
            cycles_from_samples(&wave(100), Some(4096)).unwrap().len(),
            1
        );
        // Too many frames are cut at the table's limit; bad files make none.
        assert_eq!(
            cycles_from_samples(&wave(300 * 256), Some(256))
                .unwrap()
                .len(),
            MAX_FRAMES
        );
        assert!(cycles_from_samples(&[0.5], None).is_none());
        assert!(cycles_from_samples(&[0.0, f32::NAN, 0.1], None).is_none());
        assert!(cycles_from_samples(&[], None).is_none());
        // Whatever comes out builds into a table.
        assert!(
            FrameTable::from_cycles(&four).is_some() && FrameTable::from_cycles(&odd).is_some()
        );
    }

    #[test]
    fn quantizing_keeps_a_table_to_16_bits_and_clips_past_full_scale() {
        let cycles = vec![vec![0.0, 0.5, -0.5, 1.0], vec![2.0, -3.0, 0.25, -1.0]];
        let data = quantize(&cycles);
        assert_eq!(data.len(), 8);
        let back = dequantize(&data, 4);
        assert_eq!(back.len(), 2);
        let worst = cycles
            .iter()
            .flatten()
            .zip(back.iter().flatten())
            .map(|(a, b)| (a.clamp(-1.0, 1.0) - b).abs())
            .fold(0.0, f32::max);
        assert!(worst < 1e-4, "{worst}");
        // A rest shorter than a frame is dropped.
        assert_eq!(dequantize(&data[..7], 4).len(), 1);
        assert!(dequantize(&[], 4).is_empty());
    }

    #[test]
    fn bad_sources_make_no_table_and_extra_frames_are_cut() {
        assert!(FrameTable::from_cycles(&[]).is_none());
        assert!(FrameTable::from_cycles(&[vec![1.0]]).is_none());
        assert!(FrameTable::from_cycles(&[vec![0.0, f32::NAN, 0.5]]).is_none());
        let many: Vec<Vec<f32>> = (0..MAX_FRAMES + 40).map(|_| sine(64)).collect();
        assert_eq!(FrameTable::from_cycles(&many).unwrap().frames(), MAX_FRAMES);
        // A silent frame stays silent and finite.
        let table = FrameTable::from_cycles(&[vec![0.0; 128]]).unwrap();
        assert!(table.outline(0, 64).iter().all(|v| *v == 0.0));
    }
}
