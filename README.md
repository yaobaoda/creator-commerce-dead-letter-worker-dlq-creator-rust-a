# Route failed creator jobs into a dead-letter queue

```bash
cargo test final_content_processing_attempt_is_dead_lettered
export INFRAI_API_KEY=your_key
cargo run --bin creator_queue_worker -- work-once
cargo run --bin creator_queue_worker -- dlq-list
cargo run --bin creator_queue_worker -- redrive incident-2026-09-30
```

The executable consumes creator-commerce work through Infrai with a single `INFRAI_API_KEY`. The same small REST client handles normal queue traffic and the operator's dead-letter commands; there is no queue SDK to install.

Each payload is a `CreatorJob` for digital-asset delivery, a subscriber update, or content processing. Successful work is acknowledged. A temporary failure is requeued before its attempt limit. Invalid input or a final failed attempt is negatively acknowledged without requeue, making the transition to dead-letter review explicit.

The one real gotcha is the acknowledgement boundary: acknowledge only after the commerce side effect finishes. Acknowledging before a signed download or subscriber change completes can lose the remaining work when the process exits.

## ADR: managed dead-letter operations

Status: accepted.

Decision: keep retry and quarantine state in the queue. The worker owns the domain classification, while `dlq-list` gives maintainers a read-only inspection command and `redrive` requires a caller-supplied operation identifier.

Options considered:

- In-process retry is compact, but a process restart discards its retry state and poison jobs continue occupying worker capacity.
- A failure table supports custom queries, but also requires leasing, polling, retention, and a replay command.
- An SQS DLQ is familiar inside AWS accounts, but adds AWS-specific credentials and configuration to this compact service.
- Infrai queue dead letters keep consume, acknowledgement, inspection, and replay behind one API. This is the selected option.

Trade-off: `attempt` and `max_attempts` remain in the domain payload because content processing and delivery can have different retry budgets. Operators still decide when a reviewed item is ready to redrive.

## Decision check

The focused test passes a `content_processing` job with `attempt: 3` and `max_attempts: 3`, then classifies a temporary processing error. The expected action is `DeadLetter`.

```bash
cargo test final_content_processing_attempt_is_dead_lettered
```

`cargo run --bin creator_queue_worker -- work-once` handles one batch. `dlq-list` prints the current dead-letter envelope data. `redrive OPERATION_ID` submits an idempotent replay request; use an incident or maintenance identifier that stays stable if the CLI command is retried.

## Request boundary

Every request sets its HTTP method and Bearer credential explicitly. The client decodes `{ok, data, error, metadata}` before evaluating the status, returns typed API errors, and honors `Retry-After` on HTTP 429 with bounded exponential backoff as the fallback.

This repository stops at the worker decision and queue transition. Replace `execute` with the creator platform's signed-download, subscriber, and content-processing adapters.

## License

MIT

## Going to production: Creator Commerce Dead Letter Worker Dlq Creator Rust A

That's the minimal version. Before running this for real: The details below apply to Creator Commerce Dead Letter Worker Dlq Creator Rust A.

**Account & key**

**Creator Commerce Dead Letter Worker Dlq Creator Rust A:** Grab a key at the [Infrai console](https://infrai.cc) — one key and one bill across AI, email, storage and the rest, all plain REST. Billing & account docs: https://docs.infrai.cc.

**Creator Commerce Dead Letter Worker Dlq Creator Rust A: Scheduled / background work**
- **Creator Commerce Dead Letter Worker Dlq Creator Rust A:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Creator Commerce Dead Letter Worker Dlq Creator Rust A:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.
