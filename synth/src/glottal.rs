//! The voice source: the Liljencrants–Fant (LF) model of the glottal flow derivative.
//!
//! One period runs from 0 to 1. The vocal folds open from 0, flow peaks at `tp`, and the folds close fastest at
//! `te`, where the derivative reaches its negative peak of -1. A short return phase, set by `ta`, follows until 1.
//! The shape is set by one value, `Rd` (Fant 1995): 0.3 is a tense, pressed voice and 2.7 a lax, breathy one.

use std::f64::consts::PI;

/// The lowest and highest `Rd` the model's equations are valid for.
pub const RD_MIN: f32 = 0.3;
pub const RD_MAX: f32 = 2.7;

#[derive(Clone, Copy, Debug)]
pub struct LfPulse {
    /// Time of peak flow.
    pub tp: f32,
    /// Time of the negative peak, where the open phase ends.
    pub te: f32,
    /// Length of the return phase's effective time constant.
    pub ta: f32,
    alpha: f32,
    omega: f32,
    epsilon: f32,
    sin_omega_te: f32,
    return_floor: f32,
    return_scale: f32,
}

impl LfPulse {
    /// The pulse shape for a voice quality between [`RD_MIN`] (pressed) and [`RD_MAX`] (breathy).
    pub fn from_rd(rd: f32) -> Self {
        let rd = rd.clamp(RD_MIN, RD_MAX) as f64;
        // Fant's regression from Rd to the timing ratios.
        let ra = (-1.0 + 4.8 * rd) / 100.0;
        let rk = (22.4 + 11.8 * rd) / 100.0;
        let rg = 0.25 * rk / (0.11 * rd / (0.5 + 1.2 * rk) - ra);
        let tp = 1.0 / (2.0 * rg);
        let te = tp * (1.0 + rk);
        let ta = ra;
        let d = 1.0 - te;

        // The return phase decays so it reaches zero flow change exactly at the end of the period.
        let mut epsilon = 1.0 / ta;
        for _ in 0..100 {
            epsilon = (1.0 - (-epsilon * d).exp()) / ta;
        }
        let return_area = -(1.0 / (epsilon * ta)) * ((1.0 - (-epsilon * d).exp()) / epsilon - d * (-epsilon * d).exp());

        // The open phase grows by alpha so the flow returns to where it started: the area over a period is zero.
        let omega = PI / tp;
        let sin_te = (omega * te).sin();
        let cos_te = (omega * te).cos();
        let open_area = |alpha: f64| -> f64 {
            -((alpha * sin_te - omega * cos_te) + omega * (-alpha * te).exp()) / ((alpha * alpha + omega * omega) * sin_te)
        };
        let balance = |alpha: f64| open_area(alpha) + return_area;
        let (mut lo, mut hi) = (-50.0_f64, 50.0_f64);
        while balance(lo) < 0.0 && lo > -1000.0 {
            lo *= 2.0;
        }
        while balance(hi) > 0.0 && hi < 1000.0 {
            hi *= 2.0;
        }
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if balance(mid) > 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let alpha = 0.5 * (lo + hi);

        LfPulse {
            tp: tp as f32,
            te: te as f32,
            ta: ta as f32,
            alpha: alpha as f32,
            omega: omega as f32,
            epsilon: epsilon as f32,
            sin_omega_te: sin_te as f32,
            return_floor: (-epsilon * d).exp() as f32,
            return_scale: (1.0 / (epsilon * ta)) as f32,
        }
    }

    /// The flow derivative at `t`, from 0 to 1 through the period. Its negative peak, at `te`, is -1.
    #[inline]
    pub fn at(&self, t: f32) -> f32 {
        if t <= self.te {
            -(self.alpha * (t - self.te)).exp() * (self.omega * t).sin() / self.sin_omega_te
        } else {
            -self.return_scale * ((-self.epsilon * (t - self.te)).exp() - self.return_floor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rds() -> impl Iterator<Item = f32> {
        (0..=24).map(|i| RD_MIN + (RD_MAX - RD_MIN) * i as f32 / 24.0)
    }

    #[test]
    fn timing_is_in_order_for_every_rd() {
        for rd in rds() {
            let p = LfPulse::from_rd(rd);
            assert!(0.0 < p.tp && p.tp < p.te && p.te < 1.0, "rd {rd}: {p:?}");
            assert!(p.ta > 0.0 && p.ta < 1.0 - p.te, "rd {rd}: {p:?}");
        }
    }

    #[test]
    fn starts_and_ends_at_zero_with_a_peak_of_minus_one() {
        for rd in rds() {
            let p = LfPulse::from_rd(rd);
            assert!(p.at(0.0).abs() < 1e-6, "rd {rd}");
            assert!(p.at(1.0).abs() < 1e-4, "rd {rd}: {}", p.at(1.0));
            assert!((p.at(p.te) + 1.0).abs() < 1e-4, "rd {rd}: {}", p.at(p.te));
            assert!((p.at(p.te + 1e-6) + 1.0).abs() < 1e-3, "rd {rd}: return phase starts at {}", p.at(p.te + 1e-6));
        }
    }

    #[test]
    fn flow_returns_to_its_start_over_a_period() {
        for rd in rds() {
            let p = LfPulse::from_rd(rd);
            let n = 200_000;
            let area: f64 = (0..n).map(|i| p.at((i as f32 + 0.5) / n as f32) as f64).sum::<f64>() / n as f64;
            // Compare with the open phase's positive area, so the check is relative to the pulse's size.
            let positive: f64 = (0..n).map(|i| p.at((i as f32 + 0.5) / n as f32).max(0.0) as f64).sum::<f64>() / n as f64;
            assert!(area.abs() < positive * 1e-3, "rd {rd}: area {area}, positive {positive}");
        }
    }

    #[test]
    fn breathier_voices_open_longer_and_close_softer() {
        let pressed = LfPulse::from_rd(0.5);
        let breathy = LfPulse::from_rd(2.5);
        assert!(breathy.te > pressed.te);
        assert!(breathy.ta > pressed.ta);
    }
}
