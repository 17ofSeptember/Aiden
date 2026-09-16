use crate::model::{Example, Profile};
use std::collections::BTreeMap;
pub fn correlation(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.;
    }
    let ma = a.iter().sum::<f64>() / a.len() as f64;
    let mb = b.iter().sum::<f64>() / b.len() as f64;
    let (mut dot, mut aa, mut bb) = (0., 0., 0.);
    for (&x, &y) in a.iter().zip(b) {
        dot += (x - ma) * (y - mb);
        aa += (x - ma).powi(2);
        bb += (y - mb).powi(2);
    }
    if aa * bb < 1e-12 {
        return if a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.01) {
            1.
        } else {
            0.
        };
    }
    (dot / (aa * bb).sqrt()).clamp(-1., 1.)
}
pub fn score(a: &Example, b: &Example, sensitivity: f64) -> f64 {
    if a.shape.len() != 64 || b.shape.len() != 64 {
        return 0.;
    }
    let corr = correlation(&a.shape, &b.shape).max(0.);
    let distance = (a
        .shape
        .iter()
        .zip(&b.shape)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        / 64.)
        .sqrt();
    let duration = (-(a.duration_ms / b.duration_ms).ln().abs() / 0.65).exp();
    let amplitude = (-(a.peak / (b.peak / sensitivity)).ln().abs() / 1.2).exp();
    ((0.55 * corr + 0.45 * (-3. * distance).exp()) * duration.powf(0.18) * amplitude.powf(0.12))
        .clamp(0., 1.)
}
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
pub fn references(profile: &Profile) -> Vec<Example> {
    let mut variants: BTreeMap<&str, Vec<&Example>> = BTreeMap::new();
    for e in profile.examples.iter().filter(|e| !e.negative) {
        variants.entry(&e.variant).or_default().push(e);
    }
    variants
        .values()
        .map(|examples| {
            let mut e = examples[0].clone();
            e.shape = (0..64)
                .map(|i| median(examples.iter().map(|e| e.shape[i]).collect()))
                .collect();
            e.duration_ms = median(examples.iter().map(|e| e.duration_ms).collect());
            e.peak = median(examples.iter().map(|e| e.peak).collect());
            e.rms = median(examples.iter().map(|e| e.rms).collect());
            e
        })
        .collect()
}
pub fn profile_score(candidate: &Example, profile: &Profile) -> f64 {
    if candidate.duration_ms < profile.min_ms as f64
        || candidate.duration_ms > profile.max_ms as f64
    {
        return 0.;
    }
    let best = references(profile)
        .iter()
        .map(|r| score(candidate, r, profile.sensitivity))
        .fold(0., f64::max);
    let negative = profile
        .examples
        .iter()
        .filter(|e| e.negative)
        .map(|r| score(candidate, r, 1.))
        .fold(0., f64::max);
    if negative >= best - 0.05 {
        0.
    } else {
        best
    }
}
pub fn ranked(candidate: &Example, profiles: &[Profile]) -> Vec<(String, f64)> {
    let mut scores: Vec<_> = profiles
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (p.id.clone(), profile_score(candidate, p)))
        .collect();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    scores
}
pub fn winner(candidate: &Example, profiles: &[Profile]) -> Option<(String, f64)> {
    let scores = ranked(candidate, profiles);
    let best = scores.first()?;
    let profile = profiles.iter().find(|p| p.id == best.0)?;
    if best.1 >= profile.threshold
        && best.1 - scores.get(1).map_or(0., |s| s.1) >= profile.ambiguity
    {
        Some(best.clone())
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::DspConfig, signal::example};
    fn target() -> Example {
        let v: Vec<_> = (0..200)
            .map(|i| 0.1 + (std::f64::consts::PI * i as f64 / 199.).sin() * 0.2)
            .collect();
        example(&v, &v, 500, 0)
    }
    #[test]
    fn amplitude_duration_and_wrong_shape() {
        let a = target();
        let mut b = a.clone();
        b.peak *= 1.5;
        b.duration_ms *= 1.2;
        assert!(score(&a, &b, 1.) > 0.8);
        b.shape.reverse();
        b.shape.iter_mut().for_each(|x| *x = 1. - *x);
        assert!(score(&a, &b, 1.) < 0.6);
    }
    #[test]
    fn ambiguity_and_negative_rejection() {
        let a = target();
        let mut p = Profile::new("A".into(), DspConfig::default());
        p.examples.push(a.clone());
        assert!(winner(&a, &[p.clone()]).is_some());
        let mut q = p.clone();
        q.id = "other".into();
        assert!(winner(&a, &[p.clone(), q]).is_none());
        let mut n = a.clone();
        n.negative = true;
        p.examples.push(n);
        assert!(winner(&a, &[p]).is_none());
    }
    #[test]
    fn median_resists_outlier() {
        let mut p = Profile::new("A".into(), DspConfig::default());
        p.examples = vec![target(); 5];
        p.examples[0].peak = 10.;
        assert!(references(&p)[0].peak < 0.4);
    }
}
