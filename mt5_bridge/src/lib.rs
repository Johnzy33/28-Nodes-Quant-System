
use iceoryx2::prelude::*;
use std::panic::catch_unwind;
use std::sync::{OnceLock, Mutex};
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::io::Write;

// 1. Define the concrete types
type ServiceType = ipc_threadsafe::Service;
type MarketPublisher = iceoryx2::port::publisher::Publisher<ServiceType, MarketTick, ()>;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, ZeroCopySend)]
pub struct MarketTick {
    pub symbol_id: u32,
    pub bid: f64,
    pub ask: f64,
    pub volume: u64,
}

// 2. Global Thread-Safe Storage (Rust 2024 friendly)
static PUBLISHER: OnceLock<MarketPublisher> = OnceLock::new();

// Store last error so callers (like the MQL5 EA) can retrieve it for diagnostics
static LAST_ERROR: OnceLock<Mutex<Option<CString>>> = OnceLock::new();

fn set_last_error(msg: &str) {
    // write to /tmp for external inspection
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/iceoryx_init.log") {
        let _ = writeln!(f, "{}", msg);
    }

    let c = CString::new(msg).unwrap_or_else(|_| CString::new("invalid error").unwrap());
    let mu = LAST_ERROR.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = mu.lock() {
        *guard = Some(c);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn GetLastInitError() -> *const c_char {
    if let Some(mu) = LAST_ERROR.get() {
        if let Ok(guard) = mu.lock() {
            if let Some(ref c) = *guard {
                return c.as_ptr();
            }
        }
    }
    ptr::null()
}

#[unsafe(no_mangle)]
pub extern "C" fn InitIceoryx() -> bool {
    let result = catch_unwind(|| {

        set_log_level(LogLevel::Trace);
        
        // If already initialized, return true immediately
        if PUBLISHER.get().is_some() {
            return true;
        }

        // Create node
        let node = match NodeBuilder::new().create::<ServiceType>() {
            Ok(n) => n,
            Err(e) => {
                let msg = format!("Node creation failed: {:?}", e);
                set_last_error(&msg);
                return false;
            }
        };

        // create/open service
        let service = match node
            .service_builder(&ServiceName::new("MarketData").unwrap())
            .publish_subscribe::<MarketTick>()
            .open_or_create()
        {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("Service open/create failed: {:?}", e);
                set_last_error(&msg);
                return false;
            }
        };

        // create publisher
        if let Ok(publ) = service.publisher_builder().create() {
            // OnceLock ensures this is only set once across all threads
            if PUBLISHER.set(publ).is_ok() {
                set_last_error("Init successful");
                return true;
            } else {
                let msg = "Publisher set failed (already set?)".to_string();
                set_last_error(&msg);
            }
        } else {
            let msg = "Publisher creation failed".to_string();
            set_last_error(&msg);
        }

        false
    });

    match result {
        Ok(success) => success,
        Err(e) => {
            set_last_error(&format!("InitIceoryx panicked: {:?}", e));
            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn SendTick(id: u32, bid: f64, ask: f64, vol: u64) {
    let _ = catch_unwind(|| {
        // log attempt
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/iceoryx_send.log") {
            let _ = writeln!(f, "SendTick called: id={} bid={} ask={} vol={}", id, bid, ask, vol);
        }

        // .get() gives us a thread-safe shared reference
        if let Some(publisher) = PUBLISHER.get() {
            match publisher.loan_uninit() {
                Ok(sample) => {
                    let tick = MarketTick {
                        symbol_id: id,
                        bid,
                        ask,
                        volume: vol,
                    };
                    match sample.write_payload(tick).send() {
                        Ok(_) => {
                            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/iceoryx_send.log") {
                                let _ = writeln!(f, "SendTick: send OK for id={}", id);
                            }
                        }
                        Err(e) => {
                            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/iceoryx_send.log") {
                                let _ = writeln!(f, "SendTick: send ERROR for id={} err={:?}", id, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/iceoryx_send.log") {
                        let _ = writeln!(f, "SendTick: loan_uninit ERROR for id={} err={:?}", id, e);
                    }
                }
            }
        } else {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/iceoryx_send.log") {
                let _ = writeln!(f, "SendTick: publisher NOT INITIALIZED");
            }
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn IsPublisherInitialized() -> bool {
    PUBLISHER.get().is_some()
}