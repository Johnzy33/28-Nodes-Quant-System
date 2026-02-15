use tokio::net::TcpListener;
use tokio::io::AsyncReadExt;
use bytemuck::{Pod, Zeroable};
use chrono::{DateTime, Utc};

// This struct MUST match the MQL5 struct alignment exactly (64 bytes)
#[repr(C, packed)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
struct MultiplexedTick {
    asset_name: [u8; 10], // "US30\0..."
    _dummy: [u8; 6],      // Padding for 8-byte alignment
    bid: f64,
    ask: f64,
    last: f64,
    volume: i64,
    time_msc: i64,
    flags: u32,
    _end_pad: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:9090";
    let listener = TcpListener::bind(addr).await?;
    println!("📡 Rust Engine listening on {}", addr);

    loop {
        // Wait for MT5 to connect
        let (mut socket, _) = listener.accept().await?;
        println!("✅ MT5 Terminal Connected!");

        tokio::spawn(async move {
            let mut buffer = [0u8; 64]; // Size of our MultiplexedTick struct

            loop {
                // Read exactly 64 bytes
                match socket.read_exact(&mut buffer).await {
                    Ok(_) => {
                        // ZERO-COPY: Cast the bytes directly to our Struct
                        let tick: &MultiplexedTick = bytemuck::from_bytes(&buffer);
                        
                        // Convert bytes to String for the Asset Name
                        let asset = String::from_utf8_lossy(&tick.asset_name)
                            .trim_matches(char::from(0))
                            .to_string();

                        println!("📈 [{}] {} | Bid: {:.2} | Vol: {}", 
                            asset, 
                            tick.time_msc, 
                            tick.bid, 
                            tick.volume
                        );
                        
                        // NEXT STEP: Send to TUI and Probabilities Engine here
                    }
                    Err(e) => {
                        println!("❌ Connection closed: {}", e);
                        break;
                    }
                }
            }
        });
    }
}