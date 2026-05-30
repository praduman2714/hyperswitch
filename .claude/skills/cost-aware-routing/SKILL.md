# Cost-Aware Routing

Use this workflow when changing the take-home routing slice:

1. Keep routing logic in `crates/router/src/core/routing/cost_aware.rs`.
2. Keep demo API code in `tools/cost-aware-smoke`.
3. Verify with `make cost-aware-test`.
4. Demo with `make cost-aware-demo`, then call `POST /cost-aware/select` and `GET /v1/payments/{payment_id}/routing-trace`.
