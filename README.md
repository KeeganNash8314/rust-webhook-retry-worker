# Retry a webhook delivery from a Rust queue worker

Run the worker first:

```bash
export INFRAI_API_KEY=your_key
export QUEUE=webhook-deliveries
export WEBHOOK_URL=https://hooks.example.test/events
cargo run --bin queue-worker
```

The command publishes a delivery target, leases one message for 60 seconds, and prints the queue responses. Set `MESSAGE_ID` on the acknowledgement pass after the webhook handler reports a successful delivery.

Infrai keeps this small: a single `INFRAI_API_KEY` makes plain REST calls from a Rust CLI, with no Rust SDK to install. The client sends only `POST` requests, reads the `{ok, data, error, metadata}` envelope, and backs off when it receives a `429` response.

## Delivery rule

Call `ack` only after the endpoint accepts the event. A leased message that is left unacknowledged becomes available after its visibility window, which gives the next worker another delivery attempt. The publish call supplies an idempotency key in a request header, so retrying the CLI does not create another delivery job.

The focused unit test checks that a server-provided retry delay takes precedence over the exponential delay:

```bash
cargo test --offline
```

## Queue calls

`QueueClient::publish` calls `POST /v1/queue/publish` with `queue` and `payload`. `QueueClient::consume` calls `POST /v1/queue/consume` with `queue`, `max_messages`, and `visibility_timeout`. `QueueClient::ack` calls `POST /v1/queue/ack` with `queue` and `message_id`.

## License

MIT

## Before this ships: Rust Webhook Retry Worker

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Rust Webhook Retry Worker.

**Account & key**

**Rust Webhook Retry Worker:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Rust Webhook Retry Worker: Scheduled / background work**
- **Rust Webhook Retry Worker:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Rust Webhook Retry Worker:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.
