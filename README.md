# Hyperswitch Cost-Aware Routing Fork

This fork adds one custom routing path to Hyperswitch: cost-aware connector selection. Given a card BIN, currency, and USD amount, it filters eligible connectors, estimates cost with `base_fee_usd + (percent_fee * amount_in_usd)`, applies a minimum success-rate floor, selects the cheapest acceptable connector, and stores a decision trace by payment ID.

## Quickstart

Run the demo API from a fresh clone:

```bash
make cost-aware-demo
```

You should see:

```text
Cost-aware demo API running at http://127.0.0.1:9090
POST /cost-aware/select
GET  /v1/payments/{payment_id}/routing-trace
```

This demo server reuses the real routing logic from `crates/router/src/core/routing/cost_aware.rs` and reads `config/cost_routing.toml`. It avoids compiling the full Hyperswitch router, which is heavy on low-memory machines.

## Test Payment

Create a USD routing decision for payment `pay_test_123`:

```bash
curl --location 'http://localhost:9090/cost-aware/select' \
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
curl --location 'http://localhost:9090/v1/payments/pay_test_123/routing-trace'
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

## Razorpay INR Example

Razorpay is selected when the currency is `INR` and the BIN starts with one of its configured prefixes, such as `508`.

```bash
curl --location 'http://localhost:9090/cost-aware/select' \
--header 'Content-Type: application/json' \
--data '{
  "payment_id": "pay_test_inr_508",
  "card_bin": "508999",
  "currency": "INR",
  "amount_in_usd": 100.0
}'
```

Trace the Razorpay decision:

```bash
curl --location 'http://localhost:9090/v1/payments/pay_test_inr_508/routing-trace'
```

Expected selected connector:

```json
{
  "selected_connector": "razorpay",
  "estimated_cost_usd": 2.0,
  "reason": "lowest_cost"
}
```

There is also a Postman collection at `postman/cost-aware-routing.postman_collection.json`.

## Running The Tests

```bash
make cost-aware-test
```

This runs the focused cost-aware tests and avoids compiling the full Hyperswitch router.

## Cost Config

The cost config lives at `config/cost_routing.toml`.
Add a connector by adding another `[[connectors]]` block.
`base_fee_usd` is the fixed fee, `percent_fee` is the variable fee, `supported_currencies` gates currency eligibility, and `supported_bin_prefixes` gates BIN eligibility.
An empty `supported_bin_prefixes` list means the connector accepts any BIN.
