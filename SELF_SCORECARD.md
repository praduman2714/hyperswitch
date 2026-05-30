# Self Scorecard

Working demo: 22/25

The repo has a fast demo path with `make cost-aware-demo`, one POST request to create a routing decision, and one trace request to inspect why the connector was selected. The remaining gap is that the demo is a focused harness, not the full Hyperswitch `/payments` flow.

Routing logic depth: 18/20

The rule is config-driven, uses BIN and currency eligibility, calculates connector costs, applies a success-rate floor, and falls back instead of hard-blocking when every connector is below the floor.

Test coverage: 14/15

There are tests for happy paths, floor fallback, unsupported currency, and config loading. More production coverage would include malformed config and concurrency around trace storage.

Code quality: 8/10

The core routing logic is pure and isolated. The demo API is intentionally small and uses standard-library HTTP to avoid pulling in another web framework, but it is still a demo harness rather than production server code.

Decision doc: 9/10

`DECISIONS.md` explains the build/run trade-off, mocked success rates, in-memory trace store, BIN simplification, and production next steps.

Claude Code leverage: 8/10

The repo includes `.claude` artifacts and the session transcript should show course correction around full Hyperswitch integration versus a shippable demo path.

README: 9/10

The README has a short quickstart, exact curl commands, a trace example, a Postman collection, and one focused test command.

Total: 88/100

Bonus claimed:

- Decision trace API endpoint shape: `GET /v1/payments/{payment_id}/routing-trace`.
- Load test artifact: `make cost-aware-loadtest` sends 100 RPS for 60 seconds and fails if p99 is `>= 200ms`.

What is broken or hacky:

- The demo API is not wired into the real Hyperswitch `/payments` pipeline.
- Success rates are mocked.
- Trace storage is in-memory.
- Fee config is static TOML rather than per-merchant DB-backed config.
