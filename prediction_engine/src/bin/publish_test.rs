use iceoryx2::prelude::*;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, ZeroCopySend)]
pub struct MarketTick {
    pub symbol_id: u32,
    pub bid: f64,
    pub ask: f64,
    pub volume: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Allow specifying a service name to avoid colliding with other runs (e.g., Wine DLL)
    let service_name = std::env::var("ICE_SERVICE_NAME").unwrap_or_else(|_| "MarketDataTest".to_string());
    println!("Using service name: {}", service_name);

    // Print any existing /dev/shm iox files to help diagnose IncompatibleTypes issues
    if let Ok(entries) = std::fs::read_dir("/dev/shm") {
        for e in entries.flatten().take(40) {
            if let Ok(name) = e.file_name().into_string() {
                if name.contains("iox") {
                    println!("  found: {}", name);
                }
            }
        }
    }

    let node = NodeBuilder::new().create::<ipc_threadsafe::Service>()?;
    let service = node
        .service_builder(&ServiceName::new(&service_name)?)
        .publish_subscribe::<MarketTick>()
        .open_or_create()?;

    let publisher = service.publisher_builder().create()?;

    println!("Test publisher: sending ticks (burst)...");

    // Publishing duration config: ICE_PUBLISH_DURATION seconds (default 2). Set ICE_PUBLISH_FOREVER=1 to publish until interrupted.
    let duration_secs = std::env::var("ICE_PUBLISH_DURATION").ok().and_then(|s| s.parse::<u64>().ok()).unwrap_or(2);
    let forever = std::env::var("ICE_PUBLISH_FOREVER").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);

    let mut sent = 0usize;
    if forever {
        println!("Publishing forever (Ctrl-C to stop)...");
        loop {
            if let Ok(sample) = publisher.loan_uninit() {
                let tick = MarketTick {
                    symbol_id: 9999,
                    bid: 1.2345 + (sent as f64) * 1e-6,
                    ask: 1.2346 + (sent as f64) * 1e-6,
                    volume: 42 + sent as u64,
                };
                let _ = sample.write_payload(tick).send();
                println!("Sent tick: {:?}", tick);
                sent += 1;
            } else {
                eprintln!("Failed to loan sample for publishing");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    } else {
        println!("Publishing for {} seconds...", duration_secs);
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(duration_secs) {
            if let Ok(sample) = publisher.loan_uninit() {
                let tick = MarketTick {
                    symbol_id: 9999,
                    bid: 1.2345 + (sent as f64) * 1e-6,
                    ask: 1.2346 + (sent as f64) * 1e-6,
                    volume: 42 + sent as u64,
                };
                let _ = sample.write_payload(tick).send();
                println!("Sent tick: {:?}", tick);
                sent += 1;
            } else {
                eprintln!("Failed to loan sample for publishing");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        println!("Publisher done (sent {} samples)", sent);
    }

    Ok(())
}
