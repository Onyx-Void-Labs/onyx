// Placeholder module showing integration of FSRS into a question bank.
// This file was added as part of the FSRS-6 upgrade and AI feature work.

// local stub version of scheduler; real implementation TBD
pub struct FSRS;

impl FSRS {
    pub fn next_for_retention(&self, _r: f64) -> Vec<Question> {
        Vec::new()
    }
    pub fn grade(&mut self, _qid: &str, _grade: f32) {}
}

/// Simple question representation used by the bank.
pub struct Question {
    pub id: String,
    pub text: String,
}

pub struct QuestionBank {
    pub questions: Vec<Question>,
    pub fsrs_scheduler: FSRS,
}

impl QuestionBank {
    pub fn next_review(&mut self) -> Vec<Question> {
        self.fsrs_scheduler.next_for_retention(0.9)
    }

    pub fn grade_question(&mut self, qid: &str, grade: u8) {
        self.fsrs_scheduler.grade(qid, grade as f32);
    }
}
