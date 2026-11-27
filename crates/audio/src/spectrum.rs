//! A small spectrum probe for an instrument's display: the last [`FFT`] samples through a Hann window and an FFT, folded
//! into [`BANDS`] log-spaced bands from 20 Hz to 20 kHz, as levels 0…1 for −80 … 0 dB, smoothed (quick up, slower down). A
//! band narrower than an FFT bin (the low end) reads the interpolated level at its centre, so every band moves on its own.
//!
//! Everything is allocated in [`SpectrumProbe::new`] (on the app thread); [`SpectrumProbe::push`] and the update it
//! triggers allocate nothing, so the audio thread may run them.

use std::f32::consts::TAU;

/// The FFT length: 2048 samples, 23 Hz a bin at 48 kHz and a window of 43 ms: quick enough to follow a played note.
pub const FFT: usize = 2048;
/// The bands a display gets.
pub const BANDS: usize = 96;
/// The lowest and highest frequency a band covers.
pub const LOW_HZ: f32 = 20.0;
pub const HIGH_HZ: f32 = 20_000.0;
/// The level shown as 0.
const FLOOR_DB: f32 = -80.0;
/// A new spectrum comes every this many samples.
const HOP: usize = 512;
/// How much of the old level a band keeps when it rises, and when it falls.
const RISE: f32 = 0.2;
const FALL: f32 = 0.8;

/// The spectrum probe.
pub struct SpectrumProbe {
    ring: Box<[f32]>,
    written: usize,
    since: usize,
    window: Box<[f32]>,
    cos: Box<[f32]>,
    sin: Box<[f32]>,
    reversed: Box<[u16]>,
    re: Box<[f32]>,
    im: Box<[f32]>,
    /// Each band's first and last-plus-one FFT bin, and its centre in bins.
    bins: [(usize, usize, f32); BANDS],
    /// The scale that makes a full-scale sine 0 dB.
    norm: f32,
    levels: [f32; BANDS],
}

impl SpectrumProbe {
    pub fn new(sample_rate: f32) -> Self {
        let window: Vec<f32> = (0..FFT)
            .map(|i| 0.5 - 0.5 * (TAU * i as f32 / FFT as f32).cos())
            .collect();
        let sum: f32 = window.iter().sum();
        let bits = FFT.trailing_zeros();
        let reversed: Vec<u16> = (0..FFT)
            .map(|i| ((i as u32).reverse_bits() >> (32 - bits)) as u16)
            .collect();
        let bin_hz = sample_rate.max(1.0) / FFT as f32;
        let edge = |k: usize| LOW_HZ * (HIGH_HZ / LOW_HZ).powf(k as f32 / BANDS as f32);
        let mut bins = [(0, 1, 0.0); BANDS];
        for (k, slot) in bins.iter_mut().enumerate() {
            let low = ((edge(k) / bin_hz).floor() as usize).clamp(1, FFT / 2 - 1);
            let high = ((edge(k + 1) / bin_hz).ceil() as usize).clamp(low + 1, FFT / 2);
            let centre = (edge(k) * edge(k + 1)).sqrt() / bin_hz;
            *slot = (low, high, centre.clamp(1.0, (FFT / 2 - 1) as f32));
        }
        Self {
            ring: vec![0.0; FFT].into(),
            written: 0,
            since: 0,
            window: window.into(),
            cos: (0..FFT / 2)
                .map(|i| (TAU * i as f32 / FFT as f32).cos())
                .collect(),
            sin: (0..FFT / 2)
                .map(|i| (TAU * i as f32 / FFT as f32).sin())
                .collect(),
            reversed: reversed.into(),
            re: vec![0.0; FFT].into(),
            im: vec![0.0; FFT].into(),
            bins,
            norm: 2.0 / sum,
            levels: [0.0; BANDS],
        }
    }

    /// Takes one sample of the signal. Every [`HOP`] samples it recomputes the bands into `out` (at least [`BANDS`] long).
    pub fn push(&mut self, sample: f32, out: &mut [f32]) {
        self.ring[self.written] = sample;
        self.written = (self.written + 1) % FFT;
        self.since += 1;
        if self.since >= HOP {
            self.since = 0;
            self.update(out);
        }
    }

    /// Forgets the signal: silence, bands at 0.
    pub fn reset(&mut self, out: &mut [f32]) {
        self.ring.fill(0.0);
        self.levels = [0.0; BANDS];
        for slot in out.iter_mut().take(BANDS) {
            *slot = 0.0;
        }
    }

    fn update(&mut self, out: &mut [f32]) {
        for i in 0..FFT {
            let j = usize::from(self.reversed[i]);
            self.re[j] = self.ring[(self.written + i) % FFT] * self.window[i];
            self.im[j] = 0.0;
        }
        let mut size = 2;
        while size <= FFT {
            let (half, step) = (size / 2, FFT / size);
            for start in (0..FFT).step_by(size) {
                for k in 0..half {
                    let (c, s) = (self.cos[k * step], -self.sin[k * step]);
                    let (a, b) = (start + k, start + k + half);
                    let (re, im) = (
                        self.re[b] * c - self.im[b] * s,
                        self.re[b] * s + self.im[b] * c,
                    );
                    (self.re[b], self.im[b]) = (self.re[a] - re, self.im[a] - im);
                    (self.re[a], self.im[a]) = (self.re[a] + re, self.im[a] + im);
                }
            }
            size *= 2;
        }
        for (k, &(low, high, centre)) in self.bins.iter().enumerate() {
            let at = |bin: usize| self.re[bin].hypot(self.im[bin]);
            // A band of one or two bins reads the interpolated level at its centre; a wider one its loudest bin.
            let magnitude = if high - low <= 2 {
                let (i, f) = (centre as usize, centre.fract());
                at(i) * (1.0 - f) + at((i + 1).min(FFT / 2)) * f
            } else {
                (low..high).map(at).fold(0.0, f32::max)
            } * self.norm;
            let db = 20.0 * magnitude.max(1e-6).log10();
            let new = ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0);
            let weight = if new > self.levels[k] { RISE } else { FALL };
            let level = weight * self.levels[k] + (1.0 - weight) * new;
            // Flushed: no denormals ringing in the smoother.
            self.levels[k] = if level < 1e-6 { 0.0 } else { level };
        }
        for (slot, level) in out.iter_mut().zip(self.levels) {
            *slot = level;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The band a frequency falls in.
    fn band_of(hz: f32) -> usize {
        ((hz / LOW_HZ).ln() / (HIGH_HZ / LOW_HZ).ln() * BANDS as f32) as usize
    }

    #[test]
    fn a_sine_lights_its_own_band_and_little_else() {
        let rate = 48_000.0;
        let mut probe = SpectrumProbe::new(rate);
        let mut out = [0.0; BANDS];
        for hz in [110.0_f32, 1_000.0, 8_000.0] {
            probe.reset(&mut out);
            for n in 0..FFT * 2 {
                probe.push(0.5 * (TAU * hz * n as f32 / rate).sin(), &mut out);
            }
            let band = band_of(hz);
            let peak = out
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap()
                .0;
            assert!(
                peak.abs_diff(band) <= 1,
                "{hz} Hz: band {peak}, expected {band}"
            );
            // −6 dB on the 80 dB scale: 0.925, within the smoothing and the band's width.
            assert!(out[peak] > 0.8, "{hz} Hz: {}", out[peak]);
            // Far from the tone it is quiet.
            let far = out[(band + BANDS / 2) % BANDS];
            assert!(far < 0.4, "{hz} Hz: {far} far away");
        }
    }

    #[test]
    fn silence_reads_zero_and_the_bands_never_leave_zero_to_one() {
        let mut probe = SpectrumProbe::new(44_100.0);
        let mut out = [0.5; BANDS];
        for n in 0..FFT * 3 {
            probe.push(if n % 7 == 0 { 3.0 } else { -3.0 }, &mut out);
        }
        assert!(out.iter().all(|v| (0.0..=1.0).contains(v)));
        probe.reset(&mut out);
        for _ in 0..FFT * 3 {
            probe.push(0.0, &mut out);
        }
        assert!(out.iter().all(|v| *v == 0.0));
    }
}
