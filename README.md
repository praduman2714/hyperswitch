# Hyperswitch Cost-Aware Routing Fork

This fork adds a small cost-aware routing path on top of vanilla Hyperswitch: for a card BIN, currency, and USD amount, it evaluates configured connectors, estimates cost as `base_fee_usd + (percent_fee * amount_in_usd)`, applies a minimum success-rate floor, and keeps a routing decision trace that can be looked up later.

## Quickstart

This repo includes Docker Compose for the standard Hyperswitch stack. The compose file currently runs the published Hyperswitch router image, so it is useful for getting the local stack up quickly, but it will not include this fork's Rust changes unless you build and run the router from source. On this machine, source builds are currently blocked by missing system OpenSSL/pkg-config dependencies.

```bash
docker compose up -d
curl --fail http://localhost:8080/health
```

You also need a merchant API key and connector setup before real payment creation works. In the default local setup this is usually done through the Hyperswitch control center or seeded local config.

## Test Payment

Example request shape for a USD card payment using BIN `424242`:

```bash
curl -X POST http://localhost:8080/v1/payments \
  -H 'Content-Type: application/json' \
  -H 'api-key: <your-local-api-key>' \
  -d '{
    "amount": 10000,
    "currency": "USD",
    "confirm": true,
    "payment_method": "card",
    "payment_method_data": {
      "card": {
        "card_number": "4242424242424242",
        "card_exp_month": "12",
        "card_exp_year": "29",
        "card_cvc": "123",
        "card_holder_name": "Test User"
      }
    }
  }'
```

When the cost-aware routing decision fires, the log line should look like this:

```text
cost_aware_routing selected=stripe amount_in_usd=100.00 currency=USD card_bin=424242 estimated_cost_usd=3.20 reason=lowest_cost
```

## Routing Trace

Fetch the stored decision trace for a payment:

```bash
curl -X GET http://localhost:8080/v1/payments/pay_123/routing-trace \
  -H 'api-key: <your-local-api-key>'
```

Example response:

```json
{
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
```

If no trace exists, the endpoint returns `404` with:

```text
routing trace not found for this payment
```

## Running The Tests

```bash
cargo test cost_aware
```

## Cost Config

The cost config lives at `config/cost_routing.toml`.
Add a connector by adding another `[[connectors]]` block.
`base_fee_usd` is the fixed fee, `percent_fee` is the variable fee, `supported_currencies` gates currency eligibility, and `supported_bin_prefixes` gates BIN eligibility.
An empty `supported_bin_prefixes` list means the connector accepts any BIN.
