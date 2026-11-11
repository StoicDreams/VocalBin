//! The voice: everything that makes sound, driven by [`Controls`] that a controller changes over time.
//!
//! The layout follows Klatt's cascade-parallel synthesizer (1980):
//! - The glottal source (LF pulses plus breath noise) and aspiration noise go through a nasal pole-zero pair and
//!   five cascaded formants. This branch makes vowels, approximants, nasals, and `HH`.
//! - Frication noise goes through five parallel resonators and a bypass, each with its own level. This branch makes
//!   fricatives and the bursts of plosives, whose spectra a cascade can't shape.

use crate::filter::{AntiResonator, Glide, Resonator};
use crate::glottal::{LfPulse, RD_MAX, RD_MIN};
use crate::vowels::{BANDWIDTHS, REFERENCE_TRACT_CM, UPPER_FORMANTS};
use std::f32::consts::TAU;

/// Parameter numbers for `set` on [`crate::Synth`] and [`crate::Speaker`]. The pages use the same numbers.
pub mod param {
    /// Pitch in Hz, 50 to 500. For speech, the baseline the intonation moves around.
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
    /// Vowel lab only: time for formants to settle on a new vowel, in ms, 10 to 1000.
    pub const GLIDE_MS: u32 = 6;
    /// Output level, 0 to 1.
    pub const VOLUME: u32 = 7;
    /// Vowel lab only: vowel number in [`crate::VOWELS`].
    pub const VOWEL: u32 = 8;
    /// Vowel lab only: 1 to voice, 0 to fall silent, with a short fade either way.
    pub const VOICING: u32 = 9;
    /// Speech only: speaking rate, 0.5 (slow) to 2 (fast). Applies to the next text spoken.
    pub const RATE: u32 = 10;
    /// Speech only: how far pitch moves for stress and phrase shape, 0 (monotone) to 2. Applies to the next text.
    pub const INTONATION: u32 = 11;
}

/// What the voice is doing at one moment. Formants and pitch glide to new values; levels fade in a few ms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    /// F1 to F3 targets in Hz, for a 17.5 cm vocal tract. The voice scales them to its tract length.
    pub formants: [f32; 3],
    /// Time for formants to settle on these targets, in ms.
    pub glide_ms: f32,
    /// Glottal pulses (and the breath noise that goes with them), 0 to 1.
    pub voicing: f32,
    /// Aspiration noise through the cascade, as in `HH` and after `P`, `T`, `K`. 0 to 1.
    pub aspiration: f32,
    /// Levels of frication noise in the parallel bands at F2, F3, F4, F5, F6, and the bypass. 1 is a strong `S`.
    pub frication: [f32; 6],
    /// The nasal zero in Hz for nasal consonants, or 0 for an oral sound.
    pub nasal_zero: f32,
    /// Pitch as a multiple of the voice's baseline pitch.
    pub pitch_scale: f32,
}

impl Controls {
    /// Nothing sounding, with formants at a neutral vowel (schwa).
    pub const SILENT: Controls = Controls {
        formants: [500.0, 1500.0, 2500.0],
        glide_ms: 60.0,
        voicing: 0.0,
        aspiration: 0.0,
        frication: [0.0; 6],
        nasal_zero: 0.0,
        pitch_scale: 1.0,
    };
}

const VIBRATO_HZ: f32 = 5.5;
/// Klatt's flutter: three slow sines that keep the pitch from sounding mechanically steady.
const FLUTTER_HZ: [f32; 3] = [12.7, 7.1, 4.7];
const FLUTTER_DEPTH: f32 = 0.0035;
const OUTPUT_GAIN: f32 = 0.27;
const ASPIRATION_GAIN: f32 = 0.25;
const FRICATION_GAIN: f32 = 2.2;
/// The nasal pole, fixed, since the nasal cavity doesn't change shape.
const NASAL_POLE_HZ: f32 = 270.0;
const NASAL_BANDWIDTH: f32 = 100.0;
/// F6, used only by the parallel branch for the hiss of `S` and the bursts of `T` and `D`.
const F6_HZ: f32 = 4900.0;
const PARALLEL_BANDWIDTHS: [f32; 5] = [150.0, 200.0, 250.0, 300.0, 500.0];
/// Filters are retuned every this many samples. Formants still glide every sample.
const CONTROL_SAMPLES: u32 = 8;

pub struct Voice {
    sample_rate: f32,
    dt: f32,

    pitch: f32,
    tract_cm: f32,
    rd: f32,
    breath: f32,
    roughness: f32,
    vibrato: f32,
    volume: f32,

    controls: Controls,
    glide_omega: f32,
    pitch_glide: Glide,
    pitch_omega: f32,
    level_coef: f32,
    volume_now: f32,
    voicing_now: f32,
    aspiration_now: f32,
    frication_now: [f32; 6],

    phase: f32,
    phase_step: f32,
    pulse_amp: f32,
    pulse: LfPulse,
    pulse_rd: f32,
    vibrato_phase: f32,
    flutter_phase: [f32; 3],

    formants: [Glide; 5],
    nasal_zero: Glide,
    nasal_on: bool,
    cascade: [Resonator; 5],
    nasal_pole: Resonator,
    nasal_anti: AntiResonator,
    parallel: [Resonator; 5],
    parallel_norm: [f32; 5],
    control_count: u32,
    rng: u32,
    dc_in: f32,
    dc_out: f32,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = sample_rate.clamp(8_000.0, 192_000.0);
        let mut voice = Voice {
            sample_rate,
            dt: 1.0 / sample_rate,
            pitch: 110.0,
            tract_cm: REFERENCE_TRACT_CM,
            rd: 1.0,
            breath: 0.1,
            roughness: 0.15,
            vibrato: 0.0,
            volume: 0.8,
            controls: Controls::SILENT,
            glide_omega: Glide::omega_for(Controls::SILENT.glide_ms / 1000.0),
            pitch_glide: Glide::at(110.0),
            pitch_omega: Glide::omega_for(0.12),
            level_coef: 1.0 - (-1.0 / (0.003 * sample_rate)).exp(),
            volume_now: 0.8,
            voicing_now: 0.0,
            aspiration_now: 0.0,
            frication_now: [0.0; 6],
            phase: 0.0,
            phase_step: 110.0 / sample_rate,
            pulse_amp: 1.0,
            pulse: LfPulse::from_rd(1.0),
            pulse_rd: 1.0,
            vibrato_phase: 0.0,
            flutter_phase: [0.0, 0.3, 0.7],
            formants: [Glide::default(); 5],
            nasal_zero: Glide::at(NASAL_POLE_HZ),
            nasal_on: false,
            cascade: [Resonator::default(); 5],
            nasal_pole: Resonator::default(),
            nasal_anti: AntiResonator::default(),
            parallel: [Resonator::default(); 5],
            parallel_norm: [1.0; 5],
            control_count: 0,
            rng: 0x9E37_79B9,
            dc_in: 0.0,
            dc_out: 0.0,
        };
        voice.snap();
        voice
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Sets a voice setting from [`param`]. Returns false for numbers that aren't voice settings.
    pub fn set(&mut self, id: u32, value: f32) -> bool {
        if !value.is_finite() {
            return true;
        }
        match id {
            param::PITCH => self.pitch = value.clamp(50.0, 500.0),
            param::TRACT_CM => self.tract_cm = value.clamp(10.0, 22.0),
            param::RD => self.rd = value.clamp(RD_MIN, RD_MAX),
            param::BREATH => self.breath = value.clamp(0.0, 1.0),
            param::ROUGHNESS => self.roughness = value.clamp(0.0, 1.0),
            param::VIBRATO => self.vibrato = value.clamp(0.0, 2.0),
            param::VOLUME => self.volume = value.clamp(0.0, 1.0),
            _ => return false,
        }
        true
    }

    pub fn controls(&self) -> &Controls {
        &self.controls
    }

    pub fn set_controls(&mut self, controls: Controls) {
        if controls.glide_ms != self.controls.glide_ms {
            self.glide_omega = Glide::omega_for(controls.glide_ms / 1000.0);
        }
        self.controls = controls;
    }

    /// Sets only the voicing level, for controllers that fade it themselves.
    pub fn set_voicing(&mut self, voicing: f32) {
        self.controls.voicing = voicing;
    }

    /// Jumps formants, nasality, pitch, and levels straight to the current controls, with no glide.
    pub fn snap(&mut self) {
        let targets = self.formant_targets();
        for (glide, target) in self.formants.iter_mut().zip(targets) {
            *glide = Glide::at(target);
        }
        self.nasal_zero = Glide::at(self.nasal_target());
        self.pitch_glide = Glide::at(self.pitch * self.controls.pitch_scale);
        self.voicing_now = self.controls.voicing;
        self.aspiration_now = self.controls.aspiration;
        self.frication_now = self.controls.frication;
        self.control_count = 0;
    }

    /// The current frequency of formant `n` (0 to 4), which may still be moving toward its target.
    pub fn formant(&self, n: usize) -> f32 {
        self.formants[n].value
    }

    /// The current pitch in Hz, before jitter, flutter, and vibrato.
    pub fn pitch_now(&self) -> f32 {
        self.pitch_glide.value
    }

    fn formant_targets(&self) -> [f32; 5] {
        let scale = REFERENCE_TRACT_CM / self.tract_cm;
        let [f1, f2, f3] = self.controls.formants;
        [f1, f2, f3, UPPER_FORMANTS[0], UPPER_FORMANTS[1]].map(|f| f * scale)
    }

    fn nasal_target(&self) -> f32 {
        if self.controls.nasal_zero > 0.0 { self.controls.nasal_zero } else { NASAL_POLE_HZ }
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
        let f0 = self.pitch_glide.value * (vibrato / 12.0).exp2() * (1.0 + FLUTTER_DEPTH * flutter) * (1.0 + jitter);
        self.phase_step = (f0 / self.sample_rate).min(0.25);
        self.pulse_amp = 1.0 + self.roughness * 0.35 * self.noise();
        if (self.rd - self.pulse_rd).abs() > 1e-4 {
            self.pulse = LfPulse::from_rd(self.rd);
            self.pulse_rd = self.rd;
        }
    }

    /// Retunes every filter to the formants' current positions.
    fn retune(&mut self) {
        let sr = self.sample_rate;
        // An open glottis loses energy to the lungs, which widens the first formant.
        let widen = 1.0 + self.breath * 0.8 + (self.rd - 1.0).max(0.0) * 0.3;
        for n in 0..5 {
            let bandwidth = if n == 0 { BANDWIDTHS[0] * widen } else { BANDWIDTHS[n] };
            self.cascade[n].tune(self.formants[n].value, bandwidth, sr);
        }
        let zero = self.nasal_zero.value;
        let nasal_on = (zero - NASAL_POLE_HZ).abs() > 1.0;
        if nasal_on {
            if !self.nasal_on {
                self.nasal_pole.clear();
                self.nasal_anti.clear();
            }
            self.nasal_pole.tune(NASAL_POLE_HZ, NASAL_BANDWIDTH, sr);
            self.nasal_anti.tune(zero, NASAL_BANDWIDTH, sr);
        }
        self.nasal_on = nasal_on;
        if self.frication_now.iter().any(|&a| a > 1e-4) {
            let scale = REFERENCE_TRACT_CM / self.tract_cm;
            for k in 0..5 {
                let freq = if k < 4 { self.formants[k + 1].value } else { F6_HZ * scale };
                self.parallel[k].tune(freq, PARALLEL_BANDWIDTHS[k], sr);
                self.parallel_norm[k] = 1.0 / self.parallel[k].peak_gain(freq, sr);
            }
        }
    }

    #[inline]
    pub fn next_sample(&mut self) -> f32 {
        let c = self.controls;
        self.pitch_glide.step(self.pitch * c.pitch_scale, self.pitch_omega, self.dt);
        self.volume_now += (self.volume - self.volume_now) * self.level_coef;
        self.voicing_now += (c.voicing - self.voicing_now) * self.level_coef;
        self.aspiration_now += (c.aspiration - self.aspiration_now) * self.level_coef;
        for (now, target) in self.frication_now.iter_mut().zip(c.frication) {
            *now += (target - *now) * self.level_coef;
        }
        self.vibrato_phase = (self.vibrato_phase + VIBRATO_HZ * self.dt).fract();
        for (phase, hz) in self.flutter_phase.iter_mut().zip(FLUTTER_HZ) {
            *phase = (*phase + hz * self.dt).fract();
        }

        self.phase += self.phase_step;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.start_period();
        }

        let targets = self.formant_targets();
        for n in 0..5 {
            self.formants[n].step(targets[n], self.glide_omega, self.dt);
        }
        self.nasal_zero.step(self.nasal_target(), Glide::omega_for(0.02), self.dt);
        if self.control_count == 0 {
            self.retune();
        }
        self.control_count = (self.control_count + 1) % CONTROL_SAMPLES;

        // Breath noise is strongest while the folds are open, as air rushes through the gap.
        let open = self.phase < self.pulse.te;
        let breath = self.noise() * self.breath * 0.9 * if open { 1.0 } else { 0.35 };
        let glottal = (self.pulse.at(self.phase) * self.pulse_amp + breath) * self.voicing_now;
        let aspiration = if self.aspiration_now > 1e-4 { self.noise() * self.aspiration_now * ASPIRATION_GAIN } else { 0.0 };

        let mut out = glottal + aspiration;
        if self.nasal_on {
            out = self.nasal_anti.tick(self.nasal_pole.tick(out));
        }
        for resonator in self.cascade.iter_mut() {
            out = resonator.tick(out);
        }

        if self.frication_now.iter().any(|&a| a > 1e-4) {
            // In voiced fricatives the airflow, and so the hiss, pulses with the vocal folds.
            let pulsing = if self.voicing_now > 0.2 && !open { 0.5 } else { 1.0 };
            let noise = self.noise() * pulsing;
            let mut parallel = noise * self.frication_now[5];
            for k in 0..5 {
                let band = self.parallel[k].tick(noise);
                parallel += band * self.parallel_norm[k] * self.frication_now[k];
            }
            out += parallel * FRICATION_GAIN;
        }

        // Remove any DC offset, then limit softly so loud settings never clip harshly.
        let dc = out - self.dc_in + 0.995 * self.dc_out;
        self.dc_in = out;
        self.dc_out = dc;
        (dc * OUTPUT_GAIN).tanh() * self.volume_now
    }
}
