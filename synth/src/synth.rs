//! Stage 1, the vowel lab: one vowel at a time, with formants gliding to each new vowel.

use crate::vowels::VOWELS;
use crate::voice::{Controls, Voice, param};

pub struct Synth {
    voice: Voice,
    vowel: usize,
    glide_ms: f32,
    voicing: f32,
    level: f32,
    level_coef: f32,
    block: Vec<f32>,
}

impl Synth {
    pub fn new(sample_rate: f32) -> Self {
        let mut voice = Voice::new(sample_rate);
        let vowel = 6;
        let glide_ms = 120.0;
        voice.set_controls(Controls { formants: VOWELS[vowel].formants, glide_ms, ..Controls::SILENT });
        voice.snap();
        let level_coef = 1.0 - (-1.0 / (0.012 * voice.sample_rate())).exp();
        Synth { voice, vowel, glide_ms, voicing: 0.0, level: 0.0, level_coef, block: Vec::new() }
    }

    pub fn sample_rate(&self) -> f32 {
        self.voice.sample_rate()
    }

    /// Sets one of the [`param`] values, clamped to its range. Unknown numbers are ignored.
    pub fn set(&mut self, id: u32, value: f32) {
        if !value.is_finite() || self.voice.set(id, value) {
            return;
        }
        match id {
            param::GLIDE_MS => self.glide_ms = value.clamp(10.0, 1000.0),
            param::VOWEL => self.vowel = (value.max(0.0) as usize).min(VOWELS.len() - 1),
            param::VOICING => self.voicing = if value > 0.5 { 1.0 } else { 0.0 },
            _ => return,
        }
        let controls = Controls { formants: VOWELS[self.vowel].formants, glide_ms: self.glide_ms, voicing: self.level, ..Controls::SILENT };
        self.voice.set_controls(controls);
    }

    /// The current frequency of formant `n` (0 to 4), which may still be moving toward its target.
    pub fn formant(&self, n: usize) -> f32 {
        self.voice.formant(n)
    }

    /// Fills `out` with the next samples.
    pub fn render(&mut self, out: &mut [f32]) {
        for sample in out.iter_mut() {
            // Fade over about 12 ms, so starting and stopping never click.
            self.level += (self.voicing - self.level) * self.level_coef;
            self.voice.set_voicing(self.level);
            *sample = self.voice.next_sample();
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
    use std::f32::consts::TAU;

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
