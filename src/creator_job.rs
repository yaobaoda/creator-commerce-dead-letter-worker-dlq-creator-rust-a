use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    DigitalAssetDelivery,
    SubscriberUpdate,
    ContentProcessing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatorJob {
    pub job_id: String,
    pub creator_id: String,
    pub attempt: u8,
    pub max_attempts: u8,
    pub kind: JobKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Temporary,
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobError {
    Temporary(String),
    InvalidInput(String),
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Temporary(message) => write!(f, "temporary job failure: {message}"),
            Self::InvalidInput(message) => write!(f, "invalid job input: {message}"),
        }
    }
}

impl std::error::Error for JobError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobAction {
    Ack,
    Retry,
    DeadLetter,
}

pub fn decide_failure(job: &CreatorJob, class: FailureClass) -> JobAction {
    match class {
        FailureClass::InvalidInput => JobAction::DeadLetter,
        FailureClass::Temporary if job.attempt >= job.max_attempts => JobAction::DeadLetter,
        FailureClass::Temporary => JobAction::Retry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_content_processing_attempt_is_dead_lettered() {
        let job = CreatorJob {
            job_id: "content-804".into(),
            creator_id: "creator-19".into(),
            attempt: 3,
            max_attempts: 3,
            kind: JobKind::ContentProcessing,
        };

        assert_eq!(
            decide_failure(&job, FailureClass::Temporary),
            JobAction::DeadLetter
        );
    }
}
