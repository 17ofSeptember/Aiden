use crate::{
    model::{Example, Point, Profile},
    recognition::profile_score,
    require,
    signal::example,
    Result,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TrainingStatus {
    pub profile_id: Option<String>,
    pub stage: String,
    pub accepted: u32,
    pub rejected: u32,
    pub candidate: Option<Example>,
    pub capture: Vec<Point>,
    pub variant: String,
    pub score: f64,
}
pub struct Training {
    pub status: TrainingStatus,
    pub capture: Vec<Point>,
    pub started_ms: u64,
}
impl Default for Training {
    fn default() -> Self {
        Self {
            status: TrainingStatus {
                stage: "Idle".into(),
                variant: "Default".into(),
                ..Default::default()
            },
            capture: Vec::with_capacity(30000),
            started_ms: 0,
        }
    }
}
impl Training {
    pub fn start(
        &mut self,
        profile_id: String,
        stage: String,
        now: u64,
        variant: String,
    ) -> Result<()> {
        require(
            ["Capture", "Train", "Negative", "Validate", "Paused", "Idle"]
                .contains(&stage.as_str()),
            "Unknown training stage",
        )?;
        require(
            !variant.is_empty() && variant.len() <= 80,
            "Variant name required",
        )?;
        if self.status.profile_id.as_deref() != Some(&profile_id) {
            *self = Self::default();
        }
        if stage != "Paused" {
            self.status.candidate = None;
        }
        self.status.profile_id = Some(profile_id);
        self.status.stage = stage;
        self.status.variant = variant;
        self.started_ms = now;
        self.capture.clear();
        self.status.capture.clear();
        Ok(())
    }
    pub fn point(&mut self, p: &Point) {
        if self.status.stage == "Capture"
            && p.time_ms >= self.started_ms + 3000
            && self.capture.len() < 30000
        {
            self.capture.push(p.clone());
        }
    }
    pub fn finish(&mut self, rate: u32, onset: f64, release: f64) -> Result<()> {
        require(
            self.status.stage == "Capture",
            "Start a reference capture first",
        )?;
        require(
            !self.capture.is_empty(),
            "No capture samples. Wait for countdown and perform the contraction.",
        )?;
        let start = self
            .capture
            .iter()
            .position(|p| p.envelope >= onset)
            .ok_or_else(|| {
                crate::Error::Invalid(
                    "No contraction found; inspect electrodes or lower onset threshold".into(),
                )
            })?;
        let end = self
            .capture
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, p)| p.envelope < release)
            .map_or(self.capture.len(), |(i, _)| i);
        self.crop(start, end, rate)?;
        self.status.capture = self
            .capture
            .iter()
            .step_by((self.capture.len() / 1000).max(1))
            .cloned()
            .collect();
        self.status.stage = "Review".into();
        Ok(())
    }
    pub fn crop(&mut self, start: usize, end: usize, rate: u32) -> Result<()> {
        require(
            start < end
                && end <= self.capture.len()
                && end - start >= 10
                && end - start <= rate as usize * 10,
            "Invalid capture segment bounds",
        )?;
        let points = &self.capture[start..end];
        let env: Vec<_> = points.iter().map(|p| p.envelope).collect();
        let wave: Vec<_> = points.iter().map(|p| p.filtered).collect();
        let mut e = example(&env, &wave, rate, points[0].time_ms);
        e.variant = self.status.variant.clone();
        self.status.candidate = Some(e);
        Ok(())
    }
    pub fn candidate(&mut self, mut e: Example, p: &mut Profile) {
        if self.status.profile_id.as_deref() != Some(&p.id) {
            return;
        }
        e.variant = self.status.variant.clone();
        e.score = profile_score(&e, p);
        self.status.score = e.score;
        match self.status.stage.as_str() {
            "Train" if e.score >= p.threshold.max(0.85) => {
                if p.examples.len() < 256 {
                    p.examples.push(e.clone());
                    self.status.accepted += 1;
                }
            }
            "Train" => self.status.rejected += 1,
            "Negative" => {
                e.negative = true;
                if p.examples.len() < 256 {
                    p.examples.push(e.clone());
                    self.status.accepted += 1;
                }
            }
            "Validate" => {
                if e.score >= p.threshold {
                    p.validation.detected += 1;
                }
            }
            _ => return,
        }
        self.status.candidate = Some(e);
    }
    pub fn accept(&mut self, p: &mut Profile) -> Result<()> {
        require(
            self.status.profile_id.as_deref() == Some(&p.id),
            "Candidate belongs to another profile",
        )?;
        require(
            ["Review", "Train", "Negative", "Paused", "Ready"]
                .contains(&self.status.stage.as_str()),
            "No reviewable training candidate",
        )?;
        require(
            p.examples.len() < 256,
            "Maximum 256 examples; remove old examples first",
        )?;
        let mut e = self
            .status
            .candidate
            .clone()
            .ok_or_else(|| crate::Error::Invalid("No candidate to accept".into()))?;
        require(
            !p.examples.iter().any(|x| x.id == e.id),
            "Candidate already accepted",
        )?;
        e.variant = self.status.variant.clone();
        p.examples.push(e);
        self.status.accepted += 1;
        if self.status.stage == "Review" {
            self.status.stage = "Ready".into();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DspConfig;
    #[test]
    fn candidate_cannot_cross_profiles_or_duplicate() {
        let mut a = Profile::new("A".into(), DspConfig::default());
        let mut b = Profile::new("B".into(), DspConfig::default());
        let mut training = Training::default();
        training
            .start(a.id.clone(), "Capture".into(), 0, "Default".into())
            .expect("start");
        training.status.stage = "Review".into();
        training.status.candidate = Some(example(&[0.1; 64], &[0.1; 64], 500, 0));
        assert!(training.accept(&mut b).is_err());
        training.accept(&mut a).expect("accept");
        assert!(training.accept(&mut a).is_err());
        training
            .start(b.id.clone(), "Capture".into(), 0, "Default".into())
            .expect("switch");
        assert!(training.status.candidate.is_none());
        assert_eq!(training.status.accepted, 0);
    }
    #[test]
    fn capture_countdown_crop_and_invalid_transitions() {
        let mut training = Training::default();
        assert!(training.finish(500, 0.04, 0.02).is_err());
        training
            .start("a".into(), "Capture".into(), 0, "Default".into())
            .expect("start");
        for time in (0..4000).step_by(2) {
            let amplitude = if (3200..3600).contains(&time) {
                0.1
            } else {
                0.001
            };
            training.point(&Point {
                time_ms: time,
                raw: 0.5,
                filtered: amplitude,
                rectified: amplitude,
                envelope: amplitude,
            });
        }
        assert_eq!(training.capture.len(), 500);
        training.finish(500, 0.04, 0.02).expect("finish");
        assert_eq!(
            training
                .status
                .candidate
                .as_ref()
                .expect("candidate")
                .duration_ms,
            400.
        );
        assert!(training.crop(20, 10, 500).is_err());
        assert!(training.crop(0, 501, 500).is_err());
    }
}
