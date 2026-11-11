//! The vocal tract: formant resonators, and the trajectories formants follow between targets.

use std::f32::consts::PI;

/// A two-pole resonator (Klatt 1980) with a gain of 1 at 0 Hz, so a cascade of them keeps the source's level.
#[derive(Clone, Copy, Debug, Default)]
pub struct Resonator {
    a: f32,
    b: f32,
    c: f32,
    y1: f32,
    y2: f32,
    bypass: bool,
}

impl Resonator {
    /// Tunes the resonator to `freq` Hz with `bandwidth` Hz. A frequency too close to Nyquist is passed through.
    pub fn tune(&mut self, freq: f32, bandwidth: f32, sample_rate: f32) {
        self.bypass = freq >= sample_rate * 0.45;
        if self.bypass {
            return;
        }
        let t = 1.0 / sample_rate;
        let r = (-PI * bandwidth * t).exp();
        self.c = -r * r;
        self.b = 2.0 * r * (2.0 * PI * freq * t).cos();
        self.a = 1.0 - self.b - self.c;
    }

    #[inline]
    pub fn tick(&mut self, x: f32) -> f32 {
        if self.bypass {
            return x;
        }
        let y = self.a * x + self.b * self.y1 + self.c * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

impl Resonator {
    /// The gain at the resonator's own frequency, to scale a parallel branch so each band peaks at its amplitude.
    pub fn peak_gain(&self, freq: f32, sample_rate: f32) -> f32 {
        if self.bypass {
            return 1.0;
        }
        let w = 2.0 * PI * freq / sample_rate;
        // |1 - b e^-jw - c e^-2jw|
        let re = 1.0 - self.b * w.cos() - self.c * (2.0 * w).cos();
        let im = self.b * w.sin() + self.c * (2.0 * w).sin();
        self.a / (re * re + im * im).sqrt().max(1e-9)
    }

    pub fn clear(&mut self) {
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// The inverse of a [`Resonator`]: a notch, used for the zero the nasal cavity adds to nasal consonants.
#[derive(Clone, Copy, Debug, Default)]
pub struct AntiResonator {
    a: f32,
    b: f32,
    c: f32,
    x1: f32,
    x2: f32,
}

impl AntiResonator {
    pub fn tune(&mut self, freq: f32, bandwidth: f32, sample_rate: f32) {
        let mut r = Resonator::default();
        r.tune(freq.min(sample_rate * 0.45 - 1.0), bandwidth, sample_rate);
        self.a = 1.0 / r.a;
        self.b = -r.b / r.a;
        self.c = -r.c / r.a;
    }

    #[inline]
    pub fn tick(&mut self, x: f32) -> f32 {
        let y = self.a * x + self.b * self.x1 + self.c * self.x2;
        self.x2 = self.x1;
        self.x1 = x;
        y
    }

    pub fn clear(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
    }
}

/// A value that moves toward its target like a critically damped mass on a spring: it speeds up, then settles
/// without overshoot. Articulators move this way, which a straight line between targets does not.
#[derive(Clone, Copy, Debug, Default)]
pub struct Glide {
    pub value: f32,
    velocity: f32,
}

impl Glide {
    pub fn at(value: f32) -> Self {
        Glide { value, velocity: 0.0 }
    }

    /// The spring rate that settles within about 1% of a new target after `seconds`.
    pub fn omega_for(seconds: f32) -> f32 {
        6.64 / seconds.max(0.001)
    }

    #[inline]
    pub fn step(&mut self, target: f32, omega: f32, dt: f32) -> f32 {
        let accel = omega * omega * (target - self.value) - 2.0 * omega * self.velocity;
        self.velocity += accel * dt;
        self.value += self.velocity * dt;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gain_at(freq: f32, res_freq: f32, bw: f32) -> f32 {
        let sr = 44_100.0;
        let mut r = Resonator::default();
        r.tune(res_freq, bw, sr);
        let mut peak = 0.0_f32;
        for i in 0..44_100 {
            let y = r.tick((2.0 * PI * freq * i as f32 / sr).sin());
            if i > 22_050 {
                peak = peak.max(y.abs());
            }
        }
        peak
    }

    #[test]
    fn resonator_passes_low_frequencies_at_unity() {
        assert!((gain_at(5.0, 700.0, 80.0) - 1.0).abs() < 0.05);
    }

    #[test]
    fn resonator_boosts_its_formant_and_cuts_far_above_it() {
        let peak = gain_at(700.0, 700.0, 80.0);
        assert!(peak > 3.0, "peak gain {peak}");
        assert!(gain_at(4000.0, 700.0, 80.0) < 0.1);
    }

    #[test]
    fn peak_gain_matches_the_measured_peak() {
        let mut r = Resonator::default();
        r.tune(2500.0, 200.0, 44_100.0);
        let predicted = r.peak_gain(2500.0, 44_100.0);
        let measured = gain_at(2500.0, 2500.0, 200.0);
        assert!((predicted / measured - 1.0).abs() < 0.02, "predicted {predicted}, measured {measured}");
    }

    #[test]
    fn anti_resonator_notches_its_frequency_and_undoes_a_resonator() {
        let sr = 44_100.0;
        let mut res = Resonator::default();
        let mut anti = AntiResonator::default();
        res.tune(1500.0, 100.0, sr);
        anti.tune(1500.0, 100.0, sr);
        let (mut notch, mut both) = (0.0_f32, 0.0_f32);
        let mut lone = AntiResonator::default();
        lone.tune(1500.0, 100.0, sr);
        for i in 0..44_100 {
            let x = (2.0 * PI * 1500.0 * i as f32 / sr).sin();
            let y = lone.tick(x);
            let z = anti.tick(res.tick(x));
            if i > 22_050 {
                notch = notch.max(y.abs());
                both = both.max(z.abs());
            }
        }
        // A zero with a 100 Hz bandwidth sits just inside the unit circle, so it cuts about 19 dB, not to silence.
        assert!(notch < 0.15, "notch passes {notch}");
        assert!((both - 1.0).abs() < 0.02, "pair gives {both}");
    }

    #[test]
    fn glide_settles_on_time_without_overshoot() {
        let dt = 1.0 / 44_100.0;
        let omega = Glide::omega_for(0.1);
        let mut g = Glide::at(500.0);
        let mut highest = 0.0_f32;
        for _ in 0..(0.05 / dt) as usize {
            highest = highest.max(g.step(1000.0, omega, dt));
        }
        assert!(g.value < 990.0, "should still be moving halfway through, at {}", g.value);
        for _ in 0..(0.05 / dt) as usize {
            highest = highest.max(g.step(1000.0, omega, dt));
        }
        assert!((g.value - 1000.0).abs() < 10.0, "settled at {}", g.value);
        for _ in 0..(0.5 / dt) as usize {
            highest = highest.max(g.step(1000.0, omega, dt));
        }
        assert!(highest <= 1000.01, "overshot to {highest}");
    }
}
