use crate::{require, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct DspConfig {
    pub sample_rate: u32,
    pub high_pass: f64,
    pub low_pass: f64,
    pub notch: f64,
    pub envelope_ms: f64,
    pub baseline_ms: f64,
    pub onset: f64,
    pub release: f64,
    pub min_ms: u32,
    pub max_ms: u32,
    pub cooldown_ms: u32,
}
impl Default for DspConfig {
    fn default() -> Self {
        Self {
            sample_rate: 500,
            high_pass: 5.,
            low_pass: 100.,
            notch: 0.,
            envelope_ms: 35.,
            baseline_ms: 1500.,
            onset: 0.045,
            release: 0.022,
            min_ms: 100,
            max_ms: 5000,
            cooldown_ms: 350,
        }
    }
}
impl DspConfig {
    pub fn validate(&self) -> Result<()> {
        require(
            [250, 500, 1000].contains(&self.sample_rate),
            "Sample rate must be 250, 500 or 1000 Hz",
        )?;
        let nyquist = self.sample_rate as f64 / 2.;
        require(
            [
                self.high_pass,
                self.low_pass,
                self.notch,
                self.envelope_ms,
                self.baseline_ms,
                self.onset,
                self.release,
            ]
            .iter()
            .all(|v| v.is_finite()),
            "DSP values must be finite",
        )?;
        require(
            self.high_pass >= 0.
                && self.high_pass < nyquist
                && self.low_pass >= 0.
                && self.low_pass < nyquist,
            "Cutoffs must be below Nyquist",
        )?;
        require(
            self.low_pass == 0. || self.high_pass < self.low_pass,
            "High pass must be below low pass",
        )?;
        require(
            [0., 50., 60.].contains(&self.notch),
            "Notch must be disabled, 50 or 60 Hz",
        )?;
        require(
            (1. ..=1000.).contains(&self.envelope_ms)
                && (100. ..=10000.).contains(&self.baseline_ms),
            "Invalid smoothing window",
        )?;
        require(
            self.release > 0. && self.onset > self.release && self.onset <= 1.,
            "Thresholds must satisfy 0 < release < onset <= 1",
        )?;
        require(
            self.min_ms >= 20
                && self.max_ms > self.min_ms
                && self.max_ms <= 10000
                && self.cooldown_ms <= 60000,
            "Invalid duration limits",
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub sequence: u32,
    pub hardware_us: u32,
    pub acquired_us: u64,
    pub channel: u8,
    pub adc: u16,
    pub leads: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Point {
    pub time_ms: u64,
    pub raw: f64,
    pub filtered: f64,
    pub rectified: f64,
    pub envelope: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Example {
    pub id: String,
    pub captured_ms: u64,
    pub duration_ms: f64,
    pub peak: f64,
    pub rms: f64,
    pub shape: Vec<f64>,
    pub waveform: Vec<f64>,
    pub variant: String,
    pub negative: bool,
    pub score: f64,
}
impl Example {
    pub fn validate(&self) -> Result<()> {
        require(
            uuid::Uuid::parse_str(&self.id).is_ok()
                && !self.variant.is_empty()
                && self.variant.len() <= 80,
            "Invalid example identity or variant",
        )?;
        require(
            self.shape.len() == 64
                && self.waveform.len() <= 10000
                && self
                    .shape
                    .iter()
                    .chain(&self.waveform)
                    .all(|x| x.is_finite() && x.abs() <= 2.),
            "Invalid example samples",
        )?;
        require(
            self.duration_ms.is_finite()
                && self.duration_ms > 0.
                && self.duration_ms <= 10000.
                && self.peak.is_finite()
                && self.peak > 0.
                && self.rms.is_finite()
                && self.rms >= 0.
                && self.score.is_finite(),
            "Invalid example metrics",
        )
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Validation {
    pub detected: u32,
    pub missed: u32,
    pub false_triggers: u32,
    pub latency_sum_ms: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub notes: String,
    pub color: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub enabled: bool,
    pub dsp: DspConfig,
    pub sensitivity: f64,
    pub threshold: f64,
    pub ambiguity: f64,
    pub cooldown_ms: u32,
    pub min_ms: u32,
    pub max_ms: u32,
    pub mode: EventMode,
    pub examples: Vec<Example>,
    pub validation: Validation,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum EventMode {
    Discrete,
    Hold,
    Continuous,
}
impl Profile {
    pub fn new(name: String, dsp: DspConfig) -> Self {
        let now = epoch_ms();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: String::new(),
            notes: String::new(),
            color: "#E01824".into(),
            created_at: now,
            updated_at: now,
            enabled: true,
            dsp,
            sensitivity: 1.,
            threshold: 0.8,
            ambiguity: 0.08,
            cooldown_ms: 400,
            min_ms: 100,
            max_ms: 5000,
            mode: EventMode::Discrete,
            examples: vec![],
            validation: Validation::default(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        require(
            self.validation.latency_sum_ms.is_finite() && self.validation.latency_sum_ms >= 0.,
            "Invalid validation latency",
        )?;
        require(
            uuid::Uuid::parse_str(&self.id).is_ok()
                && !self.name.trim().is_empty()
                && self.name.len() <= 100
                && self.notes.len() <= 10000
                && self.description.len() <= 2000,
            "Invalid profile metadata",
        )?;
        self.dsp.validate()?;
        require(
            (0.5..=2.).contains(&self.sensitivity)
                && (0.5..=1.).contains(&self.threshold)
                && (0.01..=0.5).contains(&self.ambiguity),
            "Invalid recognition settings",
        )?;
        require(
            self.examples.len() <= 256
                && self.min_ms >= 20
                && self.max_ms <= 10000
                && self.min_ms < self.max_ms
                && self.cooldown_ms <= 60000,
            "Profile limits exceeded",
        )?;
        let mut example_ids = std::collections::BTreeSet::new();
        for example in &self.examples {
            require(
                example_ids.insert(&example.id),
                "Duplicate training example IDs",
            )?;
            example.validate()?;
        }
        Ok(())
    }
}
pub fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}
