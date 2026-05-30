# Load Test Result

Command:

```bash
make cost-aware-loadtest
```

Result:

```text
sent_requests=6000
successful_requests=6000
failed_requests=0
achieved_rps=99.91
p50_ms=13
p95_ms=25
p99_ms=30
max_ms=36
result=PASS p99_ms=30 < 200
```

Environment: local cost-aware demo API on `127.0.0.1:9090`.
