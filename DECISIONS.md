# Decisions

I built cost-aware connector routing that picks the cheapest connector for a given card BIN, currency, and amount, but only if that connector meets a minimum success-rate threshold. If every eligible connector is below the threshold, the router still picks the cheapest one instead of blocking the payment. That is intentional: a temporarily bad success-rate window should degrade routing quality, not turn into a hard payment outage.

I used static TOML for fees because I did not want to add a DB migration for a 2 hour task. The config is easy to read, easy to change, and makes the assumptions explicit. In production, this should be per-merchant rate config pulled from the database, with merchant-specific overrides for negotiated rates, connector account differences, and payment method quirks.

For the runnable demo, I added a tiny API under `tools/cost-aware-smoke` instead of forcing the full Hyperswitch router to compile on a low-memory laptop. The demo server imports the real `cost_aware.rs` routing logic and reads the repo TOML config, then exposes the two endpoints needed for review: `POST /cost-aware/select` and `GET /v1/payments/{payment_id}/routing-trace`. This is a deliberate shippable slice: the production integration point is the payment routing stage, but the assignment can be evaluated in minutes without fighting the whole router dependency graph.

What is intentionally hacky:

- Success rates are mocked hardcoded values, not real historical data.
- The trace store is in-memory, so it disappears on restart.
- BIN matching is prefix-based, not a real BIN lookup table.
- Fee values are approximate public rates, not negotiated merchant rates.
- The demo API is a focused harness around the routing logic, not a full `/payments` integration.

What I would do with 4 more hours:

- Pull real success rates from a Redis rolling window, probably the last 15 minutes.
- Replace BIN prefix matching with a proper BIN database lookup.
- Add per-merchant fee overrides in the config.
- Expose Prometheus metrics per connector per routing decision.
- Wire this directly into Hyperswitch's payment routing stage once the full router build/runtime environment is stable.

One production thing I thought about was PII handling. The card BIN, meaning the first 6 digits, is safe to use in routing traces because it identifies the bank and card type, not the cardholder. The full card number should never touch this routing layer. The decision trace should store only the BIN-level routing inputs and the connector/cost outcome.
