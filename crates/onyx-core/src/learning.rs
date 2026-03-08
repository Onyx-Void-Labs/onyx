/// Learning-related utilities, such as audio grading and topic queries.
use std::collections::HashSet;

/// A simple automatic grader for the Feynman audio exercise.  The caller
/// provides the transcription produced by a speech model (e.g. whisper-rs)
/// and a list of keywords that describe the target concept.  The grader
/// computes a percentage overlap and converts that into a 1‑4 FSRS rating
/// where 1=again, 2=hard, 3=good, 4=easy.
pub struct FeynmanAudioGrader;

impl FeynmanAudioGrader {
    /// Grade the transcription against the provided keywords.
    ///
    /// The algorithm is intentionally lightweight: we lowercase and split
    /// the transcript on non‑alphanumeric characters, then count how many
    /// of the keywords appear in the resulting token set.  A simple
    /// threshold map produces an FSRS-style rating.
    pub fn grade(transcription: &str, keywords: &[String]) -> u8 {
        if keywords.is_empty() {
            return 1;
        }

        let tokens: HashSet<String> = transcription
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect();

        let total = keywords.len() as f64;
        let matches = keywords
            .iter()
            .filter(|kw| tokens.contains(&kw.to_lowercase()))
            .count() as f64;

        let overlap = if total > 0.0 { matches / total } else { 0.0 } * 100.0;

        // mapping: >85% = good (3), >60% = hard (2), else again (1).
        // The "easy" rating is not used by the audio grader.
        if overlap > 85.0 {
            3
        } else if overlap > 60.0 {
            2
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grader_exact_match() {
        let keys = vec!["foo".into(), "bar".into()];
        assert_eq!(FeynmanAudioGrader::grade("foo bar", &keys), 4);
    }

    #[test]
    fn grader_partial() {
        let keys = vec!["alpha".into(), "beta".into(), "gamma".into()];
        let rating = FeynmanAudioGrader::grade("alpha gamma", &keys);
        assert!(rating >= 2 && rating <= 3);
    }
}
