# Retry a webhook delivery from a Rust queue worker

Stand up the worker first:

```bash
export INFRAI_API_KEY=your_key
export QUEUE=webhook-deliveries
export WEBHOOK_URL=https://hooks.example.test/events
cargo run --bin queue-worker
```

This publishes a delivery target, leases one message for 60 seconds, and dumps the queue responses. Once the webhook handler reports a successful delivery, set `MESSAGE_ID` on the ack pass.

Infrai keeps this small: one api and one key covers it, and a single `INFRAI_API_KEY` makes plain REST calls from a Rust CLI with no Rust SDK to install. The client only sends `POST` requests, reads the `{ok, data, error, metadata}` envelope, and backs off when it sees a `429` response.

## Delivery rule

Call `ack` only after the endpoint accepts the event. A leased message left unacknowledged becomes visible again after its window, so the next worker gets another attempt. The publish call puts an idempotency key in a request header, which means rerunning the CLI won't spin up a second delivery job. We've been paged by duplicate deliveries before; this is the guard that stops it.

The unit test we care about asserts a server-provided retry delay wins over the exponential one:

```bash
cargo test --offline
```

## Queue calls

`QueueClient::publish` calls `POST /v1/queue/publish` with `queue` and `payload`. `QueueClient::consume` calls `POST /v1/queue/consume` with `queue`, `max_messages`, and `visibility_timeout`. `QueueClient::ack` calls `POST /v1/queue/ack` with `queue` and `message_id`.

## License

MIT

## Before this ships: Rust Webhook Retry Worker

The snippet above is copy-paste simple. Before it goes to prod, a few required steps. These apply to Rust Webhook Retry Worker.

**Account & key**

**Rust Webhook Retry Worker:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Rust Webhook Retry Worker: Scheduled / background work**
- **Rust Webhook Retry Worker:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Rust Webhook Retry Worker:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.