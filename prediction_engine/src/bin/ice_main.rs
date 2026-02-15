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
    // Use the thread-safe IPC service to match the publisher in the DLL
    // Allow overriding the service name for testing via ICE_SERVICE_NAME
    let service_name = std::env::var("ICE_SERVICE_NAME").unwrap_or_else(|_| "MarketDataTest".to_string());
    println!("Using service name: {}", service_name);

    let node = NodeBuilder::new().create::<ipc_threadsafe::Service>()?;
    let service = node.service_builder(&ServiceName::new(&service_name)?)
        .publish_subscribe::<MarketTick>()
        .open_or_create()?;

    let subscriber = service.subscriber_builder().create()?;

    // Print some diagnostics so we can be sure the node sees the same IPC artifacts
    println!("Listening for MT5 Ticks...");
    if let Ok(entries) = std::fs::read_dir("/dev/shm") {
        println!("/dev/shm entries (partial):");
        for e in entries.take(40) {
            if let Ok(e) = e {
                if let Ok(name) = e.file_name().into_string() {
                    if name.contains("iox") || name.contains("iceoryx") {
                        println!("  {}", name);
                    }
                }
            }
        }
    }

    // also print Wine temp if present
    if let Ok(entries) = std::fs::read_dir("~/.wine/dosdevices/c:/Temp/iceoryx2") {
        println!("Wine temp iceoryx2 files:");
        for e in entries.take(40) {
            if let Ok(e) = e {
                if let Ok(name) = e.file_name().into_string() {
                    println!("  {}", name);
                }
            }
        }
    }

    loop {
        match subscriber.receive() {
            Ok(Some(sample)) => println!("Received Tick: {:?}", *sample),
            Ok(None) => { /* no data this cycle */ },
            Err(e) => println!("Subscriber receive error: {:?}", e),
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}