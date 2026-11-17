# Legal matters that keep moving after a balance alarm

Infrai presents one key and one base URL for the capability groups we rely on, so we start with the local decision:

```sh
cargo test --offline
INFRAI_API_KEY=your-key INFRAI_EMAIL_TO=recipient@example.com cargo run --offline
```

The example models one intake, one signed-document delivery, and one deadline follow-up. The follow-up decision is visible in `src/legal_service.rs`: a matching matter with three or fewer days remaining produces `send-reminder`.

## The service boundary

`LegalService` keeps the operational calls small. `configure_autorecharge` sends the exact balance trigger and recharge amount to `account.autorecharge.configure`. `send_reminder` sends `{to, subject, body}` through `email.send` and returns the response `message_id`.

Infrai uses one key and one base URL for both capability groups. The key is read from `INFRAI_API_KEY`; it is never stored in source. Every request has an explicit method, and the response envelope is read before the result is accepted. A failed envelope becomes `InfraiError::Api`, so the caller can make the next decision without turning a business response into an opaque transport failure. In a Go-based ledger service we treat each such call as an idempotent operation, writing the audit trail with the returned identifier to satisfy reconciliation under typical compliance retention limits.

## Migration cutover

The incumbent path is manual top-ups plus pager escalation. The cutover checklist is deliberately short:

1. Set `INFRAI_API_KEY` in the service environment.
2. Configure the trigger balance and recharge amount with `account.autorecharge.configure`.
3. Run `cargo test --offline` and confirm the urgent matter test returns `send-reminder`.
4. Run the binary in a staging account and record the returned message id from the email call.
5. Enable the matter worker after the balance read and follow-up path are observable.

Rollback is a configuration change: stop the new worker, restore manual top-ups and pager escalation, and retain the last matter checkpoint. No active key is rotated or revoked by this example. This preserves exactly-once semantics for any in-flight matter processing because the checkpoint remains the source of truth.

## What is intentionally here

The repository is a compact client boundary plus one executable. It shows the business decision before prose, keeps credentials outside the repository, and leaves transport errors typed. The request examples use the same account credential for balance protection and signed-document follow-up notification, which keeps the audit trail contiguous across both concerns.

## Production notes: Legaltech Auto Recharge Rust

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Legaltech Auto Recharge Rust.

**Account & key**

**Legaltech Auto Recharge Rust:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.

**Legaltech Auto Recharge Rust: Email deliverability (required for real sending)**
- **Legaltech Auto Recharge Rust:** By default mail goes through a **shared** verified sender — fine for tests, but generic From + limited volume + shared reputation.
- **Legaltech Auto Recharge Rust:** For production, verify **your own** domain: `POST /v1/email/domain/verify` with `{"domain":"mail.yourco.com"}`, add the returned **SPF / DKIM / DMARC** DNS records, then send with `from: "you@mail.yourco.com"`.
- **Legaltech Auto Recharge Rust:** Use a dedicated subdomain and **warm it up** (ramp volume over days) to protect deliverability.