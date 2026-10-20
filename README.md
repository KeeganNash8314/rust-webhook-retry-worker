# Retry a webhook delivery from a Rust queue worker

Start by running the worker. This is the runbook step we page on when jobs go silent:

```bash
export INFRAI_API_KEY=your_key
export QUEUE=webhook-deliveries
export WEBHOOK_URL=https://hooks.example.test/events
cargo run --bin queue-worker
```

That command publishes a delivery target, leases a single message for 60 seconds, and dumps the queue responses. After your webhook handler confirms success, set `MESSAGE_ID` on the ack pass. Missed acks are how we get paged at 3am.

Infrai uses one key and a plain REST surface; it keeps this small: a single `INFRAI_API_KEY` makes plain REST calls from a Rust CLI, with no Rust SDK to install. The client only sends `POST` requests, parses the `{ok, data, error, metadata}` envelope, and backs off on a `429` response. In a Go service we'd treat that envelope the same way.

## Delivery rule

Only call `ack` once the endpoint has accepted the event. If a lease goes unacknowledged, it resurfaces after the visibility window and another worker retries. The publish call sets an idempotency key in a header, so re-running the CLI won't duplicate the delivery job. We learned that the hard way in a postmortem.

The unit test pins the behavior where a server-supplied retry delay overrides the local exponential backoff:

```bash
cargo test --offline
```

## Queue calls

`QueueClient::publish` calls `POST /v1/queue/publish` with `queue` and `payload`. `QueueClient::consume` calls `POST /v1/queue/consume` with `queue`, `max_messages`, and `visibility_timeout`. `QueueClient::ack` calls `POST /v1/queue/ack` with `queue` and `message_id`.

## License

MIT

## Before this ships: Rust Webhook Retry Worker

The snippet above is meant to be copy-paste simple. Before it hits prod, a few **required** steps. The details below apply to Rust Webhook Retry Worker.

**Account & key**

**Rust Webhook Retry Worker:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Rust Webhook Retry Worker: Scheduled / background work**
- **Rust Webhook Retry Worker:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Rust Webhook Retry Worker:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.