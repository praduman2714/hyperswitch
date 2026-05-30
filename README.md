# Hyperswitch Cost-Aware Routing Fork

This fork adds cost-aware connector routing: for a card BIN, currency, and USD amount, it estimates each eligible connector's cost using `base_fee_usd + (percent_fee * amount_in_usd)`, applies a minimum success-rate floor, selects the cheapest acceptable connector, and exposes the decision trace by payment ID.

## Quickstart

The full Hyperswitch router is large to compile on a low-memory laptop, so this repo includes a small demo API that reuses the real routing logic from `crates/router/src/core/routing/cost_aware.rs` without compiling the whole router.

```bash
cd tools/cost-aware-smoke
COST_AWARE_PORT=9091 cargo run --quiet --offline --bin server
```

You should see:

```text
Cost-aware demo API running at http://127.0.0.1:9091
POST /cost-aware/select
GET  /v1/payments/{payment_id}/routing-trace
```

## Test Payment

This creates a test routing decision for payment `pay_test_123`. The BIN is `424242`, the currency is `USD`, and the amount is `$100.00`.

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

Example response:

```json
{
  "payment_id": "pay_test_123",
  "routing_decision": {
    "selected": "stripe",
    "estimated_cost_usd": 3.2,
    "reason": "lowest_cost",
    "all_candidates": [
      {
        "name": "stripe",
        "estimated_cost_usd": 3.2,
        "success_rate": 0.95,
        "excluded_reason": null
      },
      {
        "name": "adyen",
        "estimated_cost_usd": 2.62,
        "success_rate": 0.72,
        "excluded_reason": "below_success_rate_floor"
      }
    ]
  }
}
```

## Routing Trace

Fetch the stored decision trace for the same payment:

```bash
curl --location 'http://localhost:9091/v1/payments/pay_test_123/routing-trace'
```

Example response:

```json
{
  "payment_id": "pay_test_123",
  "selected_connector": "stripe",
  "estimated_cost_usd": 3.2,
  "reason": "lowest_cost",
  "explanation": "stripe costs $3.20 with success rate 0.95 and is eligible; adyen costs $2.62 with success rate 0.72 and is excluded because below_success_rate_floor. Selected stripe because it was the cheapest connector among candidates that passed the success-rate floor.",
  "all_candidates": [
    {
      "name": "stripe",
      "estimated_cost_usd": 3.2,
      "success_rate": 0.95,
      "excluded_reason": null
    },
    {
      "name": "adyen",
      "estimated_cost_usd": 2.62,
      "success_rate": 0.72,
      "excluded_reason": "below_success_rate_floor"
    }
  ]
}
```

## Running The Tests

```bash
cargo test --manifest-path tools/cost-aware-smoke/Cargo.toml cost_aware
```

## Cost Config

The cost config lives at `config/cost_routing.toml`.
Add a connector by adding another `[[connectors]]` block.
`base_fee_usd` is the fixed fee, `percent_fee` is the variable fee, `supported_currencies` gates currency eligibility, and `supported_bin_prefixes` gates BIN eligibility.
An empty `supported_bin_prefixes` list means the connector accepts any BIN.
