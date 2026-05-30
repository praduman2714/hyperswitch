# Cost-Aware Routing Demo API

This is a small HTTP demo server for the take-home assignment. It reuses the real cost-aware routing logic from `crates/router/src/core/routing/cost_aware.rs` without compiling the full Hyperswitch router.

Run it:

```bash
cd tools/cost-aware-smoke
COST_AWARE_PORT=9091 cargo run --quiet --offline --bin server
```

Create and store a routing decision:

```bash
curl --location 'http://localhost:9091/cost-aware/select' \
--header 'Content-Type: application/json' \
--data '{
  "payment_id": "pay_test_123",
  "card_bin": "424242",
  "currency": "USD",
  "amount_in_usd": 100.0
}'
```

Read the trace:

```bash
curl --location 'http://localhost:9091/v1/payments/pay_test_123/routing-trace'
```
