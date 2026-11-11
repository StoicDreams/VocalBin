//! The stage 1 synthesizer: an LF glottal source with natural variation, through five cascaded formants.

use crate::filter::{Glide, Resonator};
use crate::glottal::{LfPulse, RD_MAX, RD_MIN};
use crate::vowels::{BANDWIDTHS, REFERENCE_TRACT_CM, UPPER_FORMANTS, VOWELS};
use std::f32::consts::TAU;

/// Parameter numbers for [`Synth::set`]. The page's `app-vowel-lab` uses the same numbers.
pub mod param {
    /// Pitch in Hz, 50 to 500.
    pub const PITCH: u32 = 0;
    /// Vocal tract length in cm, 10 to 22. Shorter tracts raise every formant, as in women's and children's voices.
    pub const TRACT_CM: u32 = 1;
    /// Voice quality as the LF model's Rd, 0.3 (pressed) to 2.7 (lax).
    pub const RD: u32 = 2;
    /// Breath noise, 0 to 1.
    pub const BREATH: u32 = 3;
    /// Jitter and shimmer, 0 to 1.
    pub const ROUGHNESS: u32 = 4;
    /// Vibrato depth in semitones, 0 to 2.
    pub const VIBRATO: u32 = 5;
    /// Time for formants to settle on a new vowel, in ms, 10 to 1000.
    pub const GLIDE_MS: u32 = 6;
    /// Output level, 0 to 1.
    pub const VOLUME: u32 = 7;
    /// Vowel number in [`crate::VOWELS`].
    pub const VOWEL: u32 = 8;
    /// 1 to voice, 0 to fall silent. The level fades over a few ms either way, so there are no clicks.
    pub const VOICING: u32 = 9;
}

const VIBRATO_HZ: f32 = 5.5;
/// Klatt's flutter: three slow sines that keep the pitch from sounding mechanically steady.
const FLUTTER_HZ: [f32; 3] = [12.7, 7.1, 4.7];
const FLUTTER_DEPTH: f32 = 0.0035;
const OUTPUT_GAIN: f32 = 0.27;

pub struct Synth {
    sample_rate: f32,
    dt: f32,

    pitch: f32,
    tract_cm: f32,
    rd: f32,
    breath: f32,
    roughness: f32,
    vibrato: f32,
    glide_omega: f32,
    volume: f32,
    vowel: usize,
    voicing: f32,

    pitch_now: f32,
    pitch_coef: f32,
    level: f32,
    level_coef: f32,
    volume_now: f32,

    phase: f32,
    phase_step: f32,
    pulse_amp: f32,
    pulse: LfPulse,
    pulse_rd: f32,
    vibrato_phase: f32,
    flutter_phase: [f32; 3],

    formants: [Glide; 5],
    resonators: [Resonator; 5],
    rng: u32,
    dc_in: f32,
    dc_out: f32,
    block: Vec<f32>,
}

impl Synth {
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = sample_rate.clamp(8_000.0, 192_000.0);
        let mut synth = Synth {
            sample_rate,
            dt: 1.0 / sample_rate,
            pitch: 110.0,
            tract_cm: REFERENCE_TRACT_CM,
            rd: 1.0,
            breath: 0.1,
            roughness: 0.15,
            vibrato: 0.0,
            glide_omega: Glide::omega_for(0.12),
            volume: 0.8,
            vowel: 6,
            voicing: 0.0,
            pitch_now: 110.0,
            pitch_coef: 1.0 - (-1.0 / (0.03 * sample_rate)).exp(),
            level: 0.0,
            level_coef: 1.0 - (-1.0 / (0.012 * sample_rate)).exp(),
            volume_now: 0.8,
            phase: 0.0,
            phase_step: 110.0 / sample_rate,
            pulse_amp: 1.0,
            pulse: LfPulse::from_rd(1.0),
            pulse_rd: 1.0,
            vibrato_phase: 0.0,
            flutter_phase: [0.0, 0.3, 0.7],
            formants: [Glide::default(); 5],
            resonators: [Resonator::default(); 5],
            rng: 0x9E37_79B9,
            dc_in: 0.0,
            dc_out: 0.0,
            block: Vec::new(),
        };
        let targets = synth.formant_targets();
        for (glide, target) in synth.formants.iter_mut().zip(targets) {
            *glide = Glide::at(target);
        }
        synth
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Sets one of the [`param`] values, clamped to its range. Unknown numbers are ignored.
    pub fn set(&mut self, id: u32, value: f32) {
        if !value.is_finite() {
            return;
        }
        match id {
            param::PITCH => self.pitch = value.clamp(50.0, 500.0),
            param::TRACT_CM => self.tract_cm = value.clamp(10.0, 22.0),
            param::RD => self.rd = value.clamp(RD_MIN, RD_MAX),
            param::BREATH => self.breath = value.clamp(0.0, 1.0),
            param::ROUGHNESS => self.roughness = value.clamp(0.0, 1.0),
            param::VIBRATO => self.vibrato = value.clamp(0.0, 2.0),
            param::GLIDE_MS => self.glide_omega = Glide::omega_for(value.clamp(10.0, 1000.0) / 1000.0),
            param::VOLUME => self.volume = value.clamp(0.0, 1.0),
            param::VOWEL => self.vowel = (value.max(0.0) as usize).min(VOWELS.len() - 1),
            param::VOICING => self.voicing = if value > 0.5 { 1.0 } else { 0.0 },
            _ => {}
        }
    }

    /// The current frequency of formant `n` (0 to 4), which may still be moving toward its target.
    pub fn formant(&self, n: usize) -> f32 {
        self.formants[n].value
    }

    fn formant_targets(&self) -> [f32; 5] {
        let scale = REFERENCE_TRACT_CM / self.tract_cm;
        let [f1, f2, f3] = VOWELS[self.vowel].formants;
        [f1, f2, f3, UPPER_FORMANTS[0], UPPER_FORMANTS[1]].map(|f| f * scale)
    }

    /// A random value from -1 to 1, roughly bell shaped.
    #[inline]
    fn noise(&mut self) -> f32 {
        let mut sum = 0.0;
        for _ in 0..3 {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 17;
            self.rng ^= self.rng << 5;
            sum += self.rng as f32 / u32::MAX as f32 * 2.0 - 1.0;
        }
        sum / 3.0
    }

    /// Picks the length, loudness, and shape of the next glottal period.
    fn start_period(&mut self) {
        let flutter: f32 = self.flutter_phase.iter().map(|p| (p * TAU).sin()).sum();
        let vibrato = self.vibrato * (self.vibrato_phase * TAU).sin();
        let jitter = self.roughness * 0.03 * self.noise();
        let f0 = self.pitch_now * (vibrato / 12.0).exp2() * (1.0 + FLUTTER_DEPTH * flutter) * (1.0 + jitter);
        self.phase_step = (f0 / self.sample_rate).min(0.25);
        self.pulse_amp = 1.0 + self.roughness * 0.35 * self.noise();
        if (self.rd - self.pulse_rd).abs() > 1e-4 {
            self.pulse = LfPulse::from_rd(self.rd);
            self.pulse_rd = self.rd;
        }
    }

    #[inline]
    fn next_sample(&mut self) -> f32 {
        self.pitch_now += (self.pitch - self.pitch_now) * self.pitch_coef;
        self.level += (self.voicing - self.level) * self.level_coef;
        self.volume_now += (self.volume - self.volume_now) * self.level_coef;
        self.vibrato_phase = (self.vibrato_phase + VIBRATO_HZ * self.dt).fract();
        for (phase, hz) in self.flutter_phase.iter_mut().zip(FLUTTER_HZ) {
            *phase = (*phase + hz * self.dt).fract();
        }

        self.phase += self.phase_step;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.start_period();
        }

        // Breath noise is strongest while the folds are open, as air rushes through the gap.
        let open = if self.phase < self.pulse.te { 1.0 } else { 0.35 };
        let breath = self.noise() * self.breath * 0.9 * open;
        let source = (self.pulse.at(self.phase) * self.pulse_amp + breath) * self.level;

        let targets = self.formant_targets();
        // An open glottis loses energy to the lungs, which widens the first formant.
        let widen = 1.0 + self.breath * 0.8 + (self.rd - 1.0).max(0.0) * 0.3;
        let mut out = source;
        for n in 0..5 {
            let freq = self.formants[n].step(targets[n], self.glide_omega, self.dt);
            let bandwidth = if n == 0 { BANDWIDTHS[0] * widen } else { BANDWIDTHS[n] };
            self.resonators[n].tune(freq, bandwidth, self.sample_rate);
            out = self.resonators[n].tick(out);
        }

        // Remove any DC offset, then limit softly so loud settings never clip harshly.
        let dc = out - self.dc_in + 0.995 * self.dc_out;
        self.dc_in = out;
        self.dc_out = dc;
        (dc * OUTPUT_GAIN).tanh() * self.volume_now
    }

    /// Fills `out` with the next samples.
    pub fn render(&mut self, out: &mut [f32]) {
        for sample in out.iter_mut() {
            *sample = self.next_sample();
        }
    }

    /// Renders `frames` samples into a buffer the synthesizer owns, for callers that can't pass memory in.
    pub fn render_block(&mut self, frames: usize) -> &[f32] {
        let mut block = std::mem::take(&mut self.block);
        block.resize(frames, 0.0);
        self.render(&mut block);
        self.block = block;
        &self.block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 44_100.0;

    fn voiced(vowel: u32) -> Synth {
        let mut s = Synth::new(SR);
        s.set(param::VOWEL, vowel as f32);
        s.set(param::GLIDE_MS, 10.0);
        s.set(param::VOICING, 1.0);
        s
    }

    fn render(s: &mut Synth, seconds: f32) -> Vec<f32> {
        let mut out = vec![0.0; (seconds * SR) as usize];
        s.render(&mut out);
        out
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
    }

    /// Power at one frequency (the Goertzel algorithm).
    fn power_at(samples: &[f32], freq: f32) -> f32 {
        let coef = 2.0 * (TAU * freq / SR).cos();
        let (mut s1, mut s2) = (0.0_f32, 0.0_f32);
        for &x in samples {
            let s0 = x + coef * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        s1 * s1 + s2 * s2 - coef * s1 * s2
    }

    /// The pitch with the strongest autocorrelation between 60 and 500 Hz.
    fn estimate_pitch(samples: &[f32]) -> f32 {
        let (min_lag, max_lag) = ((SR / 500.0) as usize, (SR / 60.0) as usize);
        let best = (min_lag..max_lag)
            .max_by(|&a, &b| {
                let score = |lag: usize| samples.iter().zip(&samples[lag..]).map(|(x, y)| x * y).sum::<f32>();
                score(a).total_cmp(&score(b))
            })
            .unwrap();
        SR / best as f32
    }

    #[test]
    fn is_silent_until_voiced_and_fades_out_after() {
        let mut s = Synth::new(SR);
        assert!(rms(&render(&mut s, 0.2)) < 1e-6);
        s.set(param::VOICING, 1.0);
        let on = render(&mut s, 0.5);
        assert!(rms(&on[on.len() / 2..]) > 0.05, "voiced level {}", rms(&on));
        s.set(param::VOICING, 0.0);
        render(&mut s, 0.3);
        assert!(rms(&render(&mut s, 0.1)) < 1e-3);
    }

    #[test]
    fn output_stays_in_range_at_every_extreme() {
        for (id, value) in [
            (param::PITCH, 50.0), (param::PITCH, 500.0), (param::RD, 0.3), (param::RD, 2.7),
            (param::BREATH, 1.0), (param::ROUGHNESS, 1.0), (param::VIBRATO, 2.0), (param::TRACT_CM, 10.0),
            (param::TRACT_CM, 22.0), (param::VOLUME, 1.0),
        ] {
            for vowel in 0..VOWELS.len() as u32 {
                let mut s = voiced(vowel);
                s.set(id, value);
                let out = render(&mut s, 0.3);
                assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.0), "param {id} = {value}, vowel {vowel}");
            }
        }
    }

    #[test]
    fn default_voice_is_loud_but_not_limited() {
        for vowel in 0..VOWELS.len() as u32 {
            let mut s = voiced(vowel);
            s.set(param::VOLUME, 1.0);
            let out = render(&mut s, 0.6);
            let peak = out[out.len() / 2..].iter().fold(0.0_f32, |m, x| m.max(x.abs()));
            assert!((0.1..0.9).contains(&peak), "vowel {} peaks at {peak}", VOWELS[vowel as usize].symbol);
        }
    }

    #[test]
    fn speaks_at_the_pitch_it_is_given() {
        for pitch in [90.0, 140.0, 220.0, 330.0] {
            let mut s = voiced(6);
            s.set(param::ROUGHNESS, 0.0);
            s.set(param::PITCH, pitch);
            let out = render(&mut s, 0.5);
            let found = estimate_pitch(&out[out.len() / 2..]);
            assert!((found / pitch - 1.0).abs() < 0.03, "asked for {pitch} Hz, heard {found} Hz");
        }
    }

    #[test]
    fn vowels_put_their_energy_at_their_formants() {
        let mut ee = voiced(0);
        let mut ah = voiced(6);
        for s in [&mut ee, &mut ah] {
            s.set(param::PITCH, 100.0);
            s.set(param::ROUGHNESS, 0.0);
        }
        let ee = render(&mut ee, 1.0);
        let ah = render(&mut ah, 1.0);
        // "ee" has a low F1 and a high F2. "ah" has a high F1 and a low F2.
        let ee_ratio = power_at(&ee, 2300.0) / power_at(&ee, 800.0);
        let ah_ratio = power_at(&ah, 2300.0) / power_at(&ah, 800.0);
        assert!(ee_ratio > ah_ratio * 30.0, "ee {ee_ratio}, ah {ah_ratio}");
    }

    #[test]
    fn formants_glide_to_a_new_vowel() {
        let mut s = voiced(6);
        s.set(param::GLIDE_MS, 200.0);
        render(&mut s, 0.1);
        s.set(param::VOWEL, 0.0);
        render(&mut s, 0.05);
        let partway = s.formant(1);
        assert!(partway > 1400.0 && partway < 2250.0, "F2 after 50 ms of 200: {partway}");
        render(&mut s, 0.3);
        assert!((s.formant(1) - 2322.0).abs() < 25.0, "F2 settled at {}", s.formant(1));
    }

    #[test]
    fn shorter_tracts_raise_the_formants() {
        let mut s = voiced(6);
        s.set(param::TRACT_CM, 14.0);
        render(&mut s, 0.3);
        assert!((s.formant(0) - 768.0 * 17.5 / 14.0).abs() < 10.0, "F1 {}", s.formant(0));
    }

    #[test]
    fn same_settings_give_the_same_sound() {
        let a = render(&mut voiced(3), 0.2);
        let b = render(&mut voiced(3), 0.2);
        assert_eq!(a, b);
    }

    #[test]
    fn ignores_values_it_cannot_use() {
        let mut s = voiced(2);
        s.set(param::PITCH, f32::NAN);
        s.set(param::VOWEL, 99.0);
        s.set(42, 1.0);
        let out = render(&mut s, 0.2);
        assert!(out.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn renders_blocks_of_any_size() {
        let mut s = voiced(4);
        assert_eq!(s.render_block(128).len(), 128);
        assert_eq!(s.render_block(4096).len(), 4096);
        assert_eq!(s.render_block(0).len(), 0);
    }
}
