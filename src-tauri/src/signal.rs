use crate::{
    model::{DspConfig, Example, Point, Sample},
    Result,
};
use std::f64::consts::PI;
#[derive(Clone, Default)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    z: [f64; 2],
}
impl Biquad {
    fn new(freq: f64, rate: f64, kind: u8) -> Self {
        if freq == 0. {
            return Self {
                b: [1., 0., 0.],
                ..Self::default()
            };
        }
        let w = 2. * PI * freq / rate;
        let c = w.cos();
        let alpha = w.sin() / (2. * if kind == 2 { 25. } else { 1. / 2f64.sqrt() });
        let b = match kind {
            0 => [(1. - c) / 2., 1. - c, (1. - c) / 2.],
            1 => [(1. + c) / 2., -(1. + c), (1. + c) / 2.],
            _ => [1., -2. * c, 1.],
        };
        Self {
            b: b.map(|v| v / (1. + alpha)),
            a: [-2. * c / (1. + alpha), (1. - alpha) / (1. + alpha)],
            z: [0.; 2],
        }
    }
    fn step(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.z[0];
        self.z[0] = self.b[1] * x - self.a[0] * y + self.z[1];
        self.z[1] = self.b[2] * x - self.a[1] * y;
        y
    }
}
pub struct Pipeline {
    pub config: DspConfig,
    hp: Biquad,
    lp: Biquad,
    notch: Biquad,
    dc: f64,
    envelope: f64,
    pub noise: f64,
    count: u64,
    active: bool,
    blocked: bool,
    cooldown: u64,
    segment: Vec<f64>,
    wave: Vec<f64>,
    start: u64,
    pub last: Point,
    pub completed: Option<Example>,
    pub released: bool,
}
impl Pipeline {
    pub fn new(config: DspConfig) -> Result<Self> {
        config.validate()?;
        let rate = config.sample_rate as f64;
        let capacity = (config.max_ms as usize * config.sample_rate as usize / 1000) + 1;
        Ok(Self {
            hp: Biquad::new(config.high_pass, rate, 1),
            lp: Biquad::new(config.low_pass, rate, 0),
            notch: Biquad::new(config.notch, rate, 2),
            config,
            dc: 0.5,
            envelope: 0.,
            noise: 0.,
            count: 0,
            active: false,
            blocked: false,
            cooldown: 0,
            segment: Vec::with_capacity(capacity),
            wave: Vec::with_capacity(capacity),
            start: 0,
            last: Point {
                time_ms: 0,
                raw: 0.,
                filtered: 0.,
                rectified: 0.,
                envelope: 0.,
            },
            completed: None,
            released: false,
        })
    }
    pub fn reset(&mut self) {
        if let Ok(new) = Self::new(self.config.clone()) {
            *self = new;
        }
    }
    pub fn ready(&self) -> bool {
        self.count >= self.config.sample_rate as u64 * 2
    }
    pub fn active(&self) -> bool {
        self.active
    }
    /// Require a fresh release/onset after arming, without discarding filter warmup.
    pub fn inhibit_until_rest(&mut self) {
        self.active = false;
        self.blocked = true;
        self.completed = None;
        self.segment.clear();
        self.wave.clear();
    }
    pub fn candidate(&self) -> Option<Example> {
        if self.active
            && self.segment.len() * 1000 / self.config.sample_rate as usize
                >= self.config.min_ms as usize
        {
            Some(example(
                &self.segment,
                &self.wave,
                self.config.sample_rate,
                self.start,
            ))
        } else {
            None
        }
    }
    pub fn step(&mut self, s: &Sample) {
        self.completed = None;
        self.released = false;
        self.count += 1;
        let rate = self.config.sample_rate as f64;
        let raw = s.adc as f64 / 1023.;
        self.dc += (raw - self.dc) * (1. - (-1000. / (self.config.baseline_ms * rate)).exp());
        let x = self.hp.step(raw - self.dc);
        let x = self.notch.step(x);
        let filtered = self.lp.step(x);
        let rectified = filtered.abs();
        self.envelope +=
            (rectified - self.envelope) * (1. - (-1000. / (self.config.envelope_ms * rate)).exp());
        if !self.active {
            self.noise += (self.envelope.min(self.config.release) - self.noise) / (rate * 2.);
        }
        let ms = self.count * 1000 / self.config.sample_rate as u64;
        self.last = Point {
            time_ms: ms,
            raw,
            filtered,
            rectified,
            envelope: self.envelope,
        };
        if s.leads != 0 || s.adc < 2 || s.adc > 1021 {
            self.active = false;
            self.blocked = true;
            self.segment.clear();
            self.wave.clear();
            return;
        }
        if self.envelope < self.config.release {
            self.blocked = false;
        }
        if !self.ready() || self.blocked {
            return;
        }
        if !self.active
            && ms >= self.cooldown
            && self.envelope >= self.config.onset.max(self.noise * 3.)
        {
            self.active = true;
            self.start = ms;
            self.segment.clear();
            self.wave.clear();
        }
        if self.active {
            if self.segment.len() < self.segment.capacity() {
                self.segment.push(self.envelope);
                self.wave.push(filtered);
            }
            if ms - self.start > self.config.max_ms as u64 {
                self.active = false;
                self.blocked = true;
                self.released = true;
                self.cooldown = ms + self.config.cooldown_ms as u64;
                return;
            }
            if self.envelope < self.config.release {
                self.active = false;
                self.released = true;
                self.cooldown = ms + self.config.cooldown_ms as u64;
                if ms - self.start >= self.config.min_ms as u64 {
                    self.completed = Some(example(
                        &self.segment,
                        &self.wave,
                        self.config.sample_rate,
                        self.start,
                    ));
                }
            }
        }
    }
}
pub fn resample(samples: &[f64], n: usize) -> Vec<f64> {
    if samples.is_empty() {
        return vec![0.; n];
    }
    (0..n)
        .map(|i| {
            let pos = i as f64 * (samples.len() - 1) as f64 / (n - 1).max(1) as f64;
            let l = pos.floor() as usize;
            let h = (l + 1).min(samples.len() - 1);
            samples[l] + (samples[h] - samples[l]) * (pos - l as f64)
        })
        .collect()
}
pub fn example(envelope: &[f64], wave: &[f64], rate: u32, start: u64) -> Example {
    let peak = envelope.iter().copied().fold(0., f64::max).max(1e-9);
    Example {
        id: uuid::Uuid::new_v4().to_string(),
        captured_ms: start,
        duration_ms: envelope.len() as f64 * 1000. / rate as f64,
        peak,
        rms: (wave.iter().map(|x| x * x).sum::<f64>() / wave.len().max(1) as f64).sqrt(),
        shape: resample(envelope, 64).iter().map(|x| x / peak).collect(),
        waveform: resample(wave, wave.len().min(512)),
        variant: "Default".into(),
        negative: false,
        score: 1.,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constant_rejects_and_filter_stable() {
        let mut p = Pipeline::new(DspConfig::default()).expect("config");
        for i in 0..10000 {
            p.step(&Sample {
                sequence: i,
                hardware_us: 0,
                acquired_us: 0,
                channel: 0,
                adc: 512,
                leads: 0,
            });
            assert!(p.completed.is_none());
            assert!(p.last.filtered.is_finite());
        }
        assert!(p.last.envelope < 0.001);
    }
    #[test]
    fn linear_resampling() {
        assert_eq!(resample(&[0., 1.], 3), vec![0., 0.5, 1.]);
    }
}
