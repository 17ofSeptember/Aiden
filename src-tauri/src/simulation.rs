use crate::{model::Sample, serial::SignalSource, Result};
use std::time::Instant;
pub struct Simulation {
    rate: u32,
    seq: u32,
    start: Instant,
    seed: u32,
    pub kind: String,
    playback: Vec<Sample>,
}
impl Simulation {
    pub fn new(rate: u32, kind: String, playback: Vec<Sample>) -> Self {
        Self {
            rate,
            seq: 0,
            start: Instant::now(),
            seed: 17,
            kind,
            playback,
        }
    }
    pub fn sample(&mut self) -> Sample {
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);
        if !self.playback.is_empty() {
            let mut s = self.playback[seq as usize % self.playback.len()].clone();
            s.sequence = seq;
            s.acquired_us = self.start.elapsed().as_micros() as u64;
            return s;
        }
        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let noise = ((self.seed >> 16) as f64 / 65535. - 0.5) * 5.;
        let t = seq as f64 / self.rate as f64;
        let phase = t % 4.;
        let amp = if self.kind == "sine" {
            80.
        } else if self.kind == "noise" {
            0.
        } else if (2.5..3.3).contains(&phase) {
            220. * (std::f64::consts::PI * (phase - 2.5) / 0.8).sin().sqrt()
        } else {
            0.
        };
        let adc = (512. + noise + amp * (2. * std::f64::consts::PI * 37. * t).sin())
            .clamp(0., 1023.) as u16;
        Sample {
            sequence: seq,
            hardware_us: seq.wrapping_mul(1_000_000 / self.rate),
            acquired_us: self.start.elapsed().as_micros() as u64,
            channel: 0,
            adc,
            leads: 0,
        }
    }
}
impl SignalSource for Simulation {
    fn read(&mut self) -> Result<Vec<Sample>> {
        let due = (self.start.elapsed().as_secs_f64() * self.rate as f64) as u64;
        let count = due.saturating_sub(self.seq as u64).min(256);
        Ok((0..count).map(|_| self.sample()).collect())
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn stop(&mut self) -> Result<()> {
        Ok(())
    }
}
