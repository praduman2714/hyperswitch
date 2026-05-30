use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

fn send_request(sequence: usize) -> Result<u128, String> {
    let body = format!(
        r#"{{
  "payment_id": "pay_load_{sequence}",
  "card_bin": "424242",
  "currency": "USD",
  "amount_in_usd": 100.0
}}"#
    );
    let request = format!(
        "POST /cost-aware/select HTTP/1.1\r\nHost: localhost:9090\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );

    let started_at = Instant::now();
    let mut stream = TcpStream::connect("127.0.0.1:9090").map_err(|error| error.to_string())?;
    stream
        .write_all(request.as_bytes())
        .map_err(|error| error.to_string())?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| error.to_string())?;

    if !response.starts_with("HTTP/1.1 200 OK") {
        return Err(response
            .lines()
            .next()
            .unwrap_or("unknown response")
            .to_string());
    }

    Ok(started_at.elapsed().as_millis())
}

fn percentile(sorted: &[u128], percentile: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }

    let index = ((sorted.len() as f64 - 1.0) * percentile).ceil() as usize;
    sorted[index]
}

fn main() {
    let rps = env_usize("RPS", 100);
    let duration_seconds = env_usize("DURATION_SECONDS", 60);
    let total_requests = rps * duration_seconds;
    let latencies = Arc::new(Mutex::new(Vec::with_capacity(total_requests)));
    let failures = Arc::new(AtomicUsize::new(0));
    let started_at = Instant::now();

    println!(
        "Load testing POST /cost-aware/select at {rps} RPS for {duration_seconds}s ({total_requests} requests)"
    );

    let mut handles = Vec::with_capacity(total_requests);
    for second in 0..duration_seconds {
        let second_started_at = Instant::now();

        for request_index in 0..rps {
            let sequence = second * rps + request_index;
            let latencies = Arc::clone(&latencies);
            let failures = Arc::clone(&failures);

            handles.push(thread::spawn(move || match send_request(sequence) {
                Ok(latency_ms) => {
                    if let Ok(mut latencies) = latencies.lock() {
                        latencies.push(latency_ms);
                    }
                }
                Err(error) => {
                    failures.fetch_add(1, Ordering::Relaxed);
                    eprintln!("request {sequence} failed: {error}");
                }
            }));
        }

        let elapsed = second_started_at.elapsed();
        if elapsed < Duration::from_secs(1) {
            thread::sleep(Duration::from_secs(1) - elapsed);
        }
    }

    for handle in handles {
        let _ = handle.join();
    }

    let mut latencies = latencies
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    latencies.sort_unstable();

    let success_count = latencies.len();
    let failure_count = failures.load(Ordering::Relaxed);
    let p50 = percentile(&latencies, 0.50);
    let p95 = percentile(&latencies, 0.95);
    let p99 = percentile(&latencies, 0.99);
    let max = latencies.last().copied().unwrap_or_default();
    let total_elapsed = started_at.elapsed().as_secs_f64();
    let achieved_rps = success_count as f64 / total_elapsed;

    println!("sent_requests={total_requests}");
    println!("successful_requests={success_count}");
    println!("failed_requests={failure_count}");
    println!("achieved_rps={achieved_rps:.2}");
    println!("p50_ms={p50}");
    println!("p95_ms={p95}");
    println!("p99_ms={p99}");
    println!("max_ms={max}");

    if failure_count == 0 && p99 < 200 {
        println!("result=PASS p99_ms={p99} < 200");
    } else {
        println!("result=FAIL p99_ms={p99}, failures={failure_count}");
        std::process::exit(1);
    }
}
