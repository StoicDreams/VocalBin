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
