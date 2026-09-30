use creator_commerce_dlq::{
    decide_failure, CreatorJob, FailureClass, InfraiQueue, JobAction, JobError, JobKind,
};
use serde_json::{json, Value};

const QUEUE: &str = "creator-commerce-jobs";

async fn execute(job: &CreatorJob) -> Result<Value, JobError> {
    match job.kind {
        JobKind::DigitalAssetDelivery => Ok(json!({ "delivery": "signed_download_ready" })),
        JobKind::SubscriberUpdate => Ok(json!({ "subscriber": "updated" })),
        JobKind::ContentProcessing => Err(JobError::Temporary(
            "content processor asked for another attempt".into(),
        )),
    }
}

async fn work_once(infrai: &InfraiQueue) -> Result<(), Box<dyn std::error::Error>> {
    for message in infrai.consume(QUEUE).await?.items {
        let job: CreatorJob = serde_json::from_value(message.payload)?;
        match execute(&job).await {
            Ok(result) => {
                infrai.ack(QUEUE, &message.message_id).await?;
                println!(
                    "{}",
                    json!({ "job_id": job.job_id, "action": "ack", "result": result })
                );
            }
            Err(error) => {
                let class = match error {
                    JobError::Temporary(_) => FailureClass::Temporary,
                    JobError::InvalidInput(_) => FailureClass::InvalidInput,
                };
                let action = decide_failure(&job, class);
                let requeue = action == JobAction::Retry;
                infrai.nack(QUEUE, &message.message_id, requeue).await?;
                println!(
                    "{}",
                    json!({ "job_id": job.job_id, "action": format!("{action:?}").to_lowercase() })
                );
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let infrai = InfraiQueue::from_env()?;
    match std::env::args().nth(1).as_deref() {
        Some("work-once") => work_once(&infrai).await?,
        Some("dlq-list") => println!("{}", infrai.list_dead_letters(QUEUE).await?),
        Some("redrive") => {
            let operation_id = std::env::args()
                .nth(2)
                .ok_or("redrive requires OPERATION_ID")?;
            println!("{}", infrai.redrive(QUEUE, &operation_id).await?);
        }
        _ => return Err("use work-once, dlq-list, or redrive OPERATION_ID".into()),
    }
    Ok(())
}
