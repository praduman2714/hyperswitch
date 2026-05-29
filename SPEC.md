# Cost-Aware Routing Spec

## Feature

Cost-Aware Routing selects the lowest estimated-cost connector for a payment while respecting basic eligibility rules.

The routing decision compares configured connectors using:

```text
estimated_cost = base_fee_usd + (percent_fee * amount_in_usd)
```

## Inputs

The routing decision receives:

- Card BIN: first 6 digits of the card number
- Currency: for example, `USD` or `INR`
- Amount in USD
- List of connectors from config

Each connector config must provide enough data to evaluate:

- Connector name
- Supported currencies
- Base fee in USD
- Percent fee
- Success-rate floor metadata or score

## Outputs

The routing decision returns:

- Selected connector name
- Estimated cost in USD
- Reason:
  - `lowest_cost`
  - `floor_fallback`
  - `only_option`
- All candidates considered, for trace and audit

Each candidate in the trace should include:

- Connector name
- Whether it supports the payment currency
- Estimated cost in USD, when calculable
- Whether it passed the success-rate floor
- Why it was selected or rejected

## Selection Rules

1. Start with the connector list from config.
2. Remove connectors that do not support the payment currency.
3. Calculate `estimated_cost` for each remaining connector.
4. Prefer connectors that pass the success-rate floor.
5. If multiple connectors pass the floor, select the cheapest one.
6. If exactly one eligible connector remains, select it with reason `only_option`.
7. If all eligible connectors fail the success-rate floor, select the cheapest eligible connector anyway with reason `floor_fallback`.

## Failure Modes

- If no connector supports the payment currency, return `RoutingError::NoEligibleConnector`.
- If all connectors fail the success-rate floor, pick the cheapest eligible connector anyway and set:

```text
reason = "floor_fallback"
```

## Files To Create

- `SPEC.md`: this specification.
- A new Rust module for the cost-aware routing decision logic, likely under `crates/router/src/core/payments/routing/`.

## Files To Update Later

- `crates/router/src/core/payments/routing.rs`: integrate the cost-aware decision into existing routing flow.
- `crates/router/src/core/payments.rs`: call the cost-aware route when the configured routing mode requires it.
- Relevant routing config/model files under `crates/api_models`, `crates/common_enums`, or `crates/diesel_models` if config types need to be persisted or exposed.
- Focused tests under `crates/router/tests` or the nearest existing routing test module.
- `README.md` or a demo document: add instructions for triggering and observing the cost-aware decision.
- `DECISIONS.md`: summarize trade-offs, skipped work, and follow-up production hardening.
