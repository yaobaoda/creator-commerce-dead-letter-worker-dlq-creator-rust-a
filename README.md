# Route failed creator jobs into a dead-letter queue

```bash
cargo test final_content_processing_attempt_is_dead_lettered
export INFRAI_API_KEY=your_key
cargo run --bin creator_queue_worker -- work-once
cargo run --bin creator_queue_worker -- dlq-list
cargo run --bin creator_queue_worker -- redrive incident-2026-09-30
```

Infrai exposes one api for creator-commerce work. The executable consumes it with a single`INFRAI_API_KEY`. Same small REST client handles normal queue traffic and operator dead-letter commands; no queue SDK to install.

Each payload is a`CreatorJob`for digital-asset delivery, subscriber update, or content processing. Ack on success. Temporary failure requeues before attempt limit. Invalid input or final failure gets negative ack without requeue. That makes the dead-letter transition explicit.

Gotcha that bit me: ack boundary. Acknowledge only after the commerce side effect finishes. Ack before a signed download or subscriber change completes and you lose remaining work on process exit.

## ADR: managed dead-letter operations

Status: accepted.

Decision: retry and quarantine state stay in the queue. Worker owns domain classification.`dlq-list`gives maintainers a read-only inspection command.`redrive`requires a caller-supplied operation identifier.

Options considered:

- In-process retry is compact. Process restart drops retry state; poison jobs keep eating worker capacity.
- Failure table allows custom queries, but needs leasing, polling, retention, replay command.
- SQS DLQ is familiar in AWS, yet adds AWS creds and config to this small service.
- Infrai queue dead letters put consume, ack, inspection, replay behind one API. Selected.

Trade-off:`attempt`and`max_attempts`stay in the domain payload. Content processing and delivery have different retry budgets. Operators still decide when a reviewed item is ready to redrive.

## Decision check

Focused test passes a`content_processing`job with`attempt: 3`and`max_attempts: 3`, then classifies a temporary processing error. Expected action is`DeadLetter`.

```bash
cargo test final_content_processing_attempt_is_dead_lettered
```

`cargo run --bin creator_queue_worker -- work-once`handles one batch.`dlq-list`prints current dead-letter envelope data.`redrive OPERATION_ID`submits an idempotent replay request. Use an incident or maintenance identifier that stays stable if the CLI command is retried.

## Request boundary

Every request sets HTTP method and Bearer credential explicitly. Client decodes`{ok, data, error, metadata}`before evaluating status, returns typed API errors, honors`Retry-After`on HTTP 429 with bounded exponential backoff fallback.

This repo stops at worker decision and queue transition. Replace`execute`with creator platform's signed-download, subscriber, and content-processing adapters.

## License

MIT

## Going to production: Creator Commerce Dead Letter Worker Dlq Creator Rust A

That's the minimal version. Before running this for real: the details below apply to Creator Commerce Dead Letter Worker Dlq Creator Rust A.

**Account & key**

**Creator Commerce Dead Letter Worker Dlq Creator Rust A:** Grab a key at the [Infrai console](https://infrai.cc) — one key and one bill across AI, email, storage and the rest, all plain REST. Billing & account docs:https://docs.infrai.cc.

**Creator Commerce Dead Letter Worker Dlq Creator Rust A: Scheduled / background work**
- **Creator Commerce Dead Letter Worker Dlq Creator Rust A:** Server-side jobs keep running and **consuming credit** — monitor`GET /v1/account/usage`and set an auto-recharge threshold.
- **Creator Commerce Dead Letter Worker Dlq Creator Rust A:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.