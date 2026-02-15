# import MetaTrader5 as mt5
# import json
# import time
# import threading
# import pytz
# import os
# import signal
# from dotenv import load_dotenv
# from pathlib import Path
# from datetime import datetime, timezone
# from kafka import KafkaConsumer, KafkaProducer


# load_dotenv()

# # --- CONFIGURATION ---
# MT5_PATH = "/home/daredevil/.wine/drive_c/Program Files/FundedNext MT5 Terminal/terminal64.exe"
# KAFKA_BROKERS = ['127.0.0.1:9092']
# CONTROL_TOPIC = "market_control"
# HEARTBEAT_TOPIC = "market_heartbeat"
# CACHED_OFFSET = None
# LAST_OFFSET_UPDATE = 0
# # HEALTH_FILE = "mt5_health.json"  # Shared with Rust Watchdog
# SOURCE_ID = "FundedNext"

# TIMEFRAME_MAP = {
#     "M1": mt5.TIMEFRAME_M1,
#     "M5": mt5.TIMEFRAME_M5,
#     "M15": mt5.TIMEFRAME_M15,
#     "H1": mt5.TIMEFRAME_H1,
#     "D1": mt5.TIMEFRAME_D1
# }

# # Thread-safe storage for active assets being streamed
# ACTIVE_STREAMS = {}
# lock = threading.Lock()

# # --- UTILITIES ---

# def get_health_path():
#     """Constructs the absolute path for the health file from .env values."""
#     base = os.getenv("MARKET_DATA_DIR", ".")
#     filename = os.getenv("HEALTH_FILE_NAME", "mt5_health.json")
#     return Path(base) / filename

# def update_health_status(is_connected):
#     """Writes status to a JSON file. Rust watchdog reads this to verify health."""
#     health_path = get_health_path()
    
#     # temp_path is created in the same folder to ensure os.replace is atomic
#     temp_path = health_path.with_suffix(".tmp")
    
#     status = {
#         "last_ping": int(time.time() * 1000),
#         "mt5_connected": is_connected,
#         "pid": os.getpid()
#     }
    
#     try:
#         # Ensure the target directory exists
#         health_path.parent.mkdir(parents=True, exist_ok=True)
        
#         with open(temp_path, "w") as f:
#             json.dump(status, f)
            
#         # Atomic swap
#         os.replace(temp_path, health_path)
#     except Exception as e:
#         print(f"[!] Health File Write Error: {e}")

# def get_kafka_producer():
#     return KafkaProducer(
#         bootstrap_servers=KAFKA_BROKERS,
#         value_serializer=lambda v: json.dumps(v).encode('utf-8'),
#         key_serializer=lambda k: k.encode('utf-8') if k else None,
#         acks=1 # Balance between speed and reliability
#     )

# # def get_broker_offset():
# #     """Calculates offset between Athens (FundedNext) and UTC."""
# #     athens_tz = pytz.timezone("Europe/Athens")
# #     now_utc = datetime.now(timezone.utc)
# #     now_naive = now_utc.replace(tzinfo=None)
# #     localized_time = athens_tz.localize(now_naive)
# #     return int(localized_time.utcoffset().total_seconds())

# def get_broker_offset():
#     global CACHED_OFFSET, LAST_OFFSET_UPDATE
#     current_time = time.time()

#     # Only refresh the offset once every 1 hour (3600 seconds)
#     if CACHED_OFFSET is None or (current_time - LAST_OFFSET_UPDATE) > 3600:
#         symbol = "EURUSD"
#         info = mt5.symbol_info_tick(symbol)
        
#         if info:
#             broker_time = info.time 
#             utc_time = int(datetime.now(timezone.utc).timestamp())
#             # Round to nearest hour
#             CACHED_OFFSET = round((broker_time - utc_time) / 3600) * 3600
#             LAST_OFFSET_UPDATE = current_time
#             print(f"🔄 [OFFSET] Refreshed Broker Offset: {CACHED_OFFSET}s")
#         else:
#             # Fallback if MT5 is busy
#             if CACHED_OFFSET is None: CACHED_OFFSET = 7200 
            
#     return CACHED_OFFSET

# def to_utc_ms(broker_time_seconds, offset):
#     return int((broker_time_seconds - offset) * 1000)

# # --- THREADED LOOPS ---

# def heartbeat_loop(producer):
#     """Handles both Kafka heartbeats and local health file updates."""
#     print("[Thread] Heartbeat & Health monitor started.")
#     while True:
#         try:
#             terminal_info = mt5.terminal_info()
#             is_connected = terminal_info is not None
            
#             # Update local health for Rust Watchdog
#             update_health_status(is_connected)

#             # Send remote heartbeat for system monitoring
#             hb_payload = {
#                 "status": "running",
#                 "mt5_connected": is_connected,
#                 "timestamp": time.time(),
#                 "source": SOURCE_ID
#             }
#             producer.send(HEARTBEAT_TOPIC, value=hb_payload)
#             time.sleep(5)
#         except Exception as e:
#             print(f"[!] Heartbeat Error: {e}")
#             time.sleep(2)


# # def perform_active_streams_backfill(producer):
# #     print("🚀 [BACKFILL] Calculating bars-ago to find the real Sunday Open...")
    
# #     # 1. Target UTC: Sunday Dec 21, 23:00:00 (which is 18:00 NY)
# #     target_utc_ts = 1766358000
# #     ts_ms = int(target_utc_ts * 1000)

# #     for sys_sym, meta in ACTIVE_STREAMS.items():
# #         broker_sym = meta['mt5_symbol']
        
# #         #
# #         rates = mt5.copy_rates_from_pos(broker_sym, mt5.TIMEFRAME_M15, 0, 100)

# #         if rates is not None and len(rates) > 0:
# #             found_bar = None
            
            
# #             for rate in rates:
                
                
# #                 # Check for the 01:00 or 01:05 Broker Time
# #                 b_dt = datetime.fromtimestamp(rate['time'], tz=timezone.utc)
                
# #                 if b_dt.hour == 1: # This is the 1:00 AM hour at the broker
# #                     found_bar = rate
# #                     break
            
# #             if found_bar:
# #                 payload = {
# #                     "asset_id": f"assets:{sys_sym.upper()}:{SOURCE_ID}",
# #                     "ts": ts_ms, # Force to the 18:00 NY slot
# #                     "open": float(found_bar['open']),
# #                     "high": float(found_bar['high']),
# #                     "low": float(found_bar['low']),
# #                     "close": float(found_bar['close']),
# #                     "volume": float(found_bar['tick_volume']),
# #                     "source": SOURCE_ID 
# #                 }
                
# #                 # Convert your forced ms back to a string to see what the DB sees
# #                 check_dt = datetime.fromtimestamp(ts_ms / 1000.0, tz=timezone.utc)
# #                 ny_dt = check_dt.astimezone(pytz.timezone("America/New_York"))
                
# #                 producer.send(f"{sys_sym.lower()}_market_data", value=payload)
# #                 print(f"✅ [FIXED] {sys_sym}: Found bar at {b_dt.strftime('%H:%M')} | Open: {found_bar['open']}")
# #                 print(f"📡 [KAFKA SEND] {sys_sym} | MS: {ts_ms}")
# #                 print(f"   -> UTC Target: {check_dt.strftime('%Y-%m-%d %H:%M:%S')}")
# #                 print(f"   -> NY Target:  {ny_dt.strftime('%Y-%m-%d %H:%M:%S %Z')}")
# #             else:
# #                 # If we still can't find it, let's print what we DID find to see the gap
# #                 earliest = datetime.fromtimestamp(rates[0]['time'], tz=timezone.utc)
# #                 print(f"❌ [FAIL] {sys_sym}: Earliest bar MT5 gave was {earliest.strftime('%H:%M')}")
# #         else:
# #             print(f"❌ [ERROR] {sys_sym}: Could not retrieve any rates.")
            
# #     producer.flush()


# # def perform_active_streams_backfill(producer):
# #     print("🚀 [BACKFILL TEST] Using Dynamic Broker Offset...")
    
# #     # Calculate current offset directly from MT5
# #     offset = get_broker_offset()
# #     print(f"ℹ️ Detected Broker Offset: {offset} seconds ({(offset/3600)}h)")

# #     for sys_sym, meta in ACTIVE_STREAMS.items():
# #         broker_sym = meta['mt5_symbol']
        
# #         # Ensure we have the start of the week in cache
# #         rates = mt5.copy_rates_from_pos(broker_sym, mt5.TIMEFRAME_M15, 0, 200)

# #         if rates is not None and len(rates) > 0:
# #             found_bar = None
# #             for rate in rates:
# #                 b_dt = datetime.fromtimestamp(rate['time'], tz=timezone.utc)
                
# #                 # We target the 01:00 AM Broker bar (18:00 NY)
# #                 if b_dt.hour == 1 and b_dt.minute == 0:
# #                     found_bar = rate
# #                     break
            
# #             if found_bar:
# #                 # Use the REAL logic to convert broker time to UTC ms
# #                 ts_ms = to_utc_ms(found_bar['time'], offset)
                
# #                 # Validation Prints
# #                 utc_dt = datetime.fromtimestamp(ts_ms / 1000.0, tz=timezone.utc)
# #                 ny_tz = pytz.timezone("America/New_York")
# #                 ny_dt = utc_dt.astimezone(ny_tz)

# #                 payload = {
# #                     "asset_id": f"assets:{sys_sym.upper()}:{SOURCE_ID}",
# #                     "ts": ts_ms,
# #                     "open": float(found_bar['open']),
# #                     "high": float(found_bar['high']),
# #                     "low": float(found_bar['low']),
# #                     "close": float(found_bar['close']),
# #                     "volume": float(found_bar['tick_volume']),
# #                     "source": SOURCE_ID 
# #                 }
                
# #                 #producer.send(f"{sys_sym.lower()}_market_data", value=payload)
# #                 print(f"✅ {sys_sym} | Broker: {b_dt.strftime('%H:%M')} | NY: {ny_dt.strftime('%H:%M')} | TS: {ts_ms}")
# #             else:
# #                 print(f"❌ {sys_sym}: Could not find 01:00 bar in history.")
# #         else:
# #             print(f"❌ {sys_sym}: No rates returned from MT5.")
            
# #     producer.flush()
    

# def stream_loop(producer):
#     """Polls MT5 and sends data ONLY if a change (Price/Volume/Time) is detected."""
#     print("[Thread] Smart Streaming loop started.")
#     last_sent_state = {}
#     #backfill_done = False

#     while True:
#         try:
            
#             # if not backfill_done and len(ACTIVE_STREAMS) > 0:
#             #     print("Waiting 10 seconds for SYNC to settle...")
#             #     time.sleep(10) # Let the Rust consumer flush all SYNC batches
#             #     perform_active_streams_backfill(producer)
#             #     backfill_done = True
                
#             offset = get_broker_offset()
#             with lock:
#                 assets = list(ACTIVE_STREAMS.items())
            
#             for sys_sym, meta in assets:
#                 # Poll ONLY the current candle (index 0)
#                 rates = mt5.copy_rates_from_pos(meta['mt5_symbol'], meta['tf'], 0, 1)
                
#                 if rates is not None and len(rates) > 0:
#                     rate = rates[0]
#                     ts_utc_ms = to_utc_ms(rate['time'], offset)
                    
#                     curr_vol = float(rate['tick_volume'])
#                     curr_close = float(rate['close'])
                    
#                     # Detect movement
#                     prev = last_sent_state.get(sys_sym)
#                     has_moved = (prev is None or 
#                                  ts_utc_ms > prev['ts'] or 
#                                  curr_vol != prev['vol'] or 
#                                  curr_close != prev['c'])

#                     if has_moved:
#                         payload = {
#                             "asset_id": f"assets:{sys_sym.upper()}:{SOURCE_ID}",
#                             "ts": ts_utc_ms,
#                             "open": float(rate['open']),
#                             "high": float(rate['high']),
#                             "low": float(rate['low']),
#                             "close": curr_close,
#                             "volume": curr_vol,
#                             "source": SOURCE_ID
#                         }
#                         producer.send(f"{sys_sym.lower()}_market_data", value=payload)
#                         last_sent_state[sys_sym] = {'ts': ts_utc_ms, 'vol': curr_vol, 'c': curr_close}
            
#             # 1s poll is industry standard for M1-M15 strategies
#             time.sleep(1) 
#         except Exception as e:
#             print(f"Stream Loop Error: {e}")
#             time.sleep(2)

# # --- COMMAND HANDLING ---

# def handle_sync(command, producer):
#     """Handles SYNC by fetching history in chunks until live."""
#     sys_sym = command['system_symbol']
#     broker_sym = command['mt5_symbol']
#     start_ts_ms = command['start_timestamp_ms']
#     tf_str = command.get('timeframe', 'M15')
#     tf = TIMEFRAME_MAP.get(tf_str, mt5.TIMEFRAME_M15)
    
#     print(f"[*] SYNC Start: {sys_sym} from {datetime.fromtimestamp(start_ts_ms/1000.0)}")

#     offset = get_broker_offset()
#     if not mt5.symbol_select(broker_sym, True):
#         print(f"[!] Symbol {broker_sym} not found.")
#         return

#     current_pointer_ms = start_ts_ms
#     now_utc_ms = int(time.time() * 1000)
#     total_bars = 0

#     # Chunked sync avoids blocking the worker for too long
#     while current_pointer_ms < (now_utc_ms - 60000):
#         start_date_naive = datetime.fromtimestamp((current_pointer_ms / 1000.0) + offset, tz=timezone.utc).replace(tzinfo=None)
#         rates = mt5.copy_rates_from(broker_sym, tf, start_date_naive, 1000)

#         if rates is None or len(rates) == 0:
#             break

#         new_max_ts = current_pointer_ms
#         for rate in rates:
#             ts_utc_ms = to_utc_ms(rate['time'], offset)
#             if ts_utc_ms > current_pointer_ms:
#                 payload = {
#                     "asset_id": f"assets:{sys_sym.upper()}:{SOURCE_ID}",
#                     "ts": ts_utc_ms,
#                     "open": float(rate['open']),
#                     "high": float(rate['high']),
#                     "low": float(rate['low']),
#                     "close": float(rate['close']),
#                     "volume": float(rate['tick_volume']),
#                     "source": SOURCE_ID
#                 }
#                 producer.send(f"{sys_sym.lower()}_market_data", value=payload)
#                 new_max_ts = max(new_max_ts, ts_utc_ms)
#                 total_bars += 1
        
#         if new_max_ts == current_pointer_ms: break
#         current_pointer_ms = new_max_ts
#         if len(rates) < 1000: break

#     with lock:
#         ACTIVE_STREAMS[sys_sym] = {"mt5_symbol": broker_sym, "last_ts": current_pointer_ms, "tf": tf}
    
#     print(f"[+] SYNC Done: {sys_sym} ({total_bars} bars). Switch to LIVE.")

# # --- MAIN ENTRY ---

# def main():
    
#     if not mt5.initialize(path=MT5_PATH):
#         print(f"FAILED to initialize MT5: {mt5.last_error()}")
#         update_health_status(False)
#         return

#     print(f"🚀 MT5 Worker Online: {SOURCE_ID}")
#     producer = get_kafka_producer()
    
#     consumer = KafkaConsumer(
#         CONTROL_TOPIC,
#         bootstrap_servers=KAFKA_BROKERS,
#         group_id=f"mt5_workerv2_{SOURCE_ID}",
#         value_deserializer=lambda v: json.loads(v.decode('utf-8'))
#     )

#     # Start Health & Streaming Threads
#     threading.Thread(target=heartbeat_loop, args=(producer,), daemon=True).start()
#     threading.Thread(target=stream_loop, args=(producer,), daemon=True).start()

#     try:
#         for message in consumer:
#             print(f"DEBUG: Received message: {message.value}")
#             cmd = message.value
#             if cmd.get("command") == "SYNC":
#                 handle_sync(cmd, producer)
#     except KeyboardInterrupt:
#         print("Stopping...")
#     finally:
#         update_health_status(False)
#         mt5.shutdown()

# if __name__ == "__main__":
#     main()

import MetaTrader5 as mt5
import json
import time
import threading
import pytz
import os
from dotenv import load_dotenv
from pathlib import Path
from datetime import datetime, timezone
from kafka import KafkaConsumer, KafkaProducer

load_dotenv()

# --- CONFIGURATION ---
MT5_PATH = os.getenv("MT5_TERMINAL_PATH", "/home/daredevil/.wine/drive_c/Program Files/FundedNext MT5 Terminal/terminal64.exe")
KAFKA_BROKERS = ['127.0.0.1:9092']
CONTROL_TOPIC = "market_control"
HEARTBEAT_TOPIC = "market_heartbeat"
SOURCE_ID = "FundedNext"

CACHED_OFFSET = None
LAST_OFFSET_UPDATE = 0

TIMEFRAME_MAP = {
    "M1": mt5.TIMEFRAME_M1,
    "M5": mt5.TIMEFRAME_M5,
    "M15": mt5.TIMEFRAME_M15,
    "H1": mt5.TIMEFRAME_H1,
    "D1": mt5.TIMEFRAME_D1
}

ACTIVE_STREAMS = {}
lock = threading.Lock()

# --- UTILITIES ---

def get_health_path():
    base = os.getenv("MARKET_DATA_DIR", ".")
    filename = os.getenv("HEALTH_FILE_NAME", "mt5_health.json")
    return Path(base) / filename

def update_health_status(is_connected):
    health_path = get_health_path()
    temp_path = health_path.with_suffix(".tmp")
    status = {
        "last_ping": int(time.time() * 1000),
        "mt5_connected": is_connected,
        "pid": os.getpid()
    }
    try:
        health_path.parent.mkdir(parents=True, exist_ok=True)
        with open(temp_path, "w") as f:
            json.dump(status, f)
        os.replace(temp_path, health_path)
    except Exception as e:
        print(f"[!] Health File Write Error: {e}")

def get_kafka_producer():
    return KafkaProducer(
        bootstrap_servers=KAFKA_BROKERS,
        value_serializer=lambda v: json.dumps(v).encode('utf-8'),
        key_serializer=lambda k: k.encode('utf-8') if k else None,
        acks=1
    )

def get_broker_offset():
    global CACHED_OFFSET, LAST_OFFSET_UPDATE
    current_time = time.time()
    if CACHED_OFFSET is None or (current_time - LAST_OFFSET_UPDATE) > 3600:
        symbol = "EURUSD"
        info = mt5.symbol_info_tick(symbol)
        if info:
            broker_time = info.time 
            utc_time = int(datetime.now(timezone.utc).timestamp())
            CACHED_OFFSET = round((broker_time - utc_time) / 3600) * 3600
            LAST_OFFSET_UPDATE = current_time
            print(f"🔄 [OFFSET] Refreshed Broker Offset: {CACHED_OFFSET}s")
        else:
            if CACHED_OFFSET is None: CACHED_OFFSET = 7200 
    return CACHED_OFFSET

def to_utc_ms(broker_time_seconds, offset):
    return int((broker_time_seconds - offset) * 1000)

# --- HEARTBEAT & STREAMING ---

def heartbeat_loop(producer):
    while True:
        try:
            terminal_info = mt5.terminal_info()
            is_connected = terminal_info is not None
            update_health_status(is_connected)
            hb_payload = {
                "status": "running",
                "mt5_connected": is_connected,
                "timestamp": time.time(),
                "source": SOURCE_ID
            }
            producer.send(HEARTBEAT_TOPIC, value=hb_payload)
            time.sleep(5)
        except Exception as e:
            print(f"[!] Heartbeat Error: {e}")
            time.sleep(2)

def stream_loop(producer):
    print("[Thread] Smart Streaming loop started.")
    last_sent_state = {}
    while True:
        try:
            offset = get_broker_offset()
            with lock:
                assets = list(ACTIVE_STREAMS.items())
            
            for sys_sym, meta in assets:
                rates = mt5.copy_rates_from_pos(meta['mt5_symbol'], meta['tf'], 0, 1)
                if rates is not None and len(rates) > 0:
                    rate = rates[0]
                    ts_utc_ms = to_utc_ms(rate['time'], offset)
                    curr_vol = float(rate['tick_volume'])
                    curr_close = float(rate['close'])
                    
                    prev = last_sent_state.get(sys_sym)
                    if (prev is None or ts_utc_ms > prev['ts'] or 
                        curr_vol != prev['vol'] or curr_close != prev['c']):
                        
                        payload = {
                            "asset_id": f"assets:{sys_sym.upper()}:{SOURCE_ID}",
                            "ts": ts_utc_ms,
                            "open": float(rate['open']),
                            "high": float(rate['high']),
                            "low": float(rate['low']),
                            "close": curr_close,
                            "volume": curr_vol,
                            "source": SOURCE_ID
                        }
                        producer.send(f"{sys_sym.lower()}_market_data", value=payload)
                        last_sent_state[sys_sym] = {'ts': ts_utc_ms, 'vol': curr_vol, 'c': curr_close}
            time.sleep(1) 
        except Exception as e:
            print(f"Stream Loop Error: {e}")
            time.sleep(2)

# --- COMMAND HANDLERS ---

def handle_sync(command, producer):
    """Fills history from a point forward and enables live streaming."""
    sys_sym = command['system_symbol']
    broker_sym = command['mt5_symbol']
    start_ts_ms = command['start_timestamp_ms']
    tf_str = command.get('timeframe', 'M15')
    tf = TIMEFRAME_MAP.get(tf_str, mt5.TIMEFRAME_M15)
    
    print(f"[*] SYNC: {sys_sym} from {datetime.fromtimestamp(start_ts_ms/1000.0, tz=timezone.utc)}")

    offset = get_broker_offset()
    if not mt5.symbol_select(broker_sym, True):
        print(f"[!] Symbol {broker_sym} not found.")
        return

    current_pointer_ms = start_ts_ms
    now_utc_ms = int(time.time() * 1000)
    total_bars = 0

    while current_pointer_ms < (now_utc_ms - 60000):
        # Convert UTC ms to broker naive datetime
        start_date = datetime.fromtimestamp((current_pointer_ms / 1000.0) + offset, tz=timezone.utc).replace(tzinfo=None)
        rates = mt5.copy_rates_from(broker_sym, tf, start_date, 1000)

        if rates is None or len(rates) == 0:
            break

        new_max_ts = current_pointer_ms
        for rate in rates:
            ts_utc_ms = to_utc_ms(rate['time'], offset)
            if ts_utc_ms > current_pointer_ms:
                payload = {
                    "asset_id": f"assets:{sys_sym.upper()}:{SOURCE_ID}",
                    "ts": ts_utc_ms,
                    "open": float(rate['open']),
                    "high": float(rate['high']),
                    "low": float(rate['low']),
                    "close": float(rate['close']),
                    "volume": float(rate['tick_volume']),
                    "source": SOURCE_ID
                }
                producer.send(f"{sys_sym.lower()}_market_data", value=payload)
                new_max_ts = max(new_max_ts, ts_utc_ms)
                total_bars += 1
        
        if new_max_ts == current_pointer_ms: break
        current_pointer_ms = new_max_ts
        if len(rates) < 1000: break

    with lock:
        ACTIVE_STREAMS[sys_sym] = {"mt5_symbol": broker_sym, "last_ts": current_pointer_ms, "tf": tf}
    
    print(f"[+] SYNC Done: {sys_sym} ({total_bars} bars).")
    producer.flush()

def handle_heal(command, producer):
    """Self-healing: Fetches a fixed window of time to fill gaps."""
    sys_sym = command['system_symbol']
    broker_sym = command['mt5_symbol']
    start_ms = command['start_timestamp_ms']
    end_ms = command.get('end_timestamp_ms', int(time.time() * 1000))
    tf = TIMEFRAME_MAP.get(command.get('timeframe', 'M15'), mt5.TIMEFRAME_M15)
    
    offset = get_broker_offset()
    
    # Convert UTC MS to Broker Local Datetime (Naive)
    from_date = datetime.fromtimestamp((start_ms / 1000.0) + offset, tz=timezone.utc).replace(tzinfo=None)
    to_date = datetime.fromtimestamp((end_ms / 1000.0) + offset, tz=timezone.utc).replace(tzinfo=None)

    print(f"🩹 [HEAL] Request for {sys_sym} window: {from_date} to {to_date}")

    rates = mt5.copy_rates_range(broker_sym, tf, from_date, to_date)

    if rates is not None and len(rates) > 0:
        for rate in rates:
            ts_utc_ms = to_utc_ms(rate['time'], offset)
            payload = {
                "asset_id": f"assets:{sys_sym.upper()}:{SOURCE_ID}",
                "ts": ts_utc_ms,
                "open": float(rate['open']),
                "high": float(rate['high']),
                "low": float(rate['low']),
                "close": float(rate['close']),
                "volume": float(rate['tick_volume']),
                "source": SOURCE_ID
            }
            producer.send(f"{sys_sym.lower()}_market_data", value=payload)
        print(f"✅ [HEAL] Dispatched {len(rates)} bars for {sys_sym}")
    else:
        print(f"⚠️ [HEAL] No data found for {sys_sym} in requested range.")
    producer.flush()

# --- MAIN ENTRY ---

def main():
    if not mt5.initialize(path=MT5_PATH):
        print(f"FAILED to initialize MT5: {mt5.last_error()}")
        update_health_status(False)
        return

    print(f"🚀 MT5 Worker Online: {SOURCE_ID}")
    producer = get_kafka_producer()
    
    # consumer = KafkaConsumer(
    #     CONTROL_TOPIC,
    #     bootstrap_servers=KAFKA_BROKERS,
    #     group_id=f"mt5_worker_{SOURCE_ID}",
    #     value_deserializer=lambda v: json.loads(v.decode('utf-8'))
    # )
    
    # consumer = KafkaConsumer(
    #     CONTROL_TOPIC,
    #     bootstrap_servers=KAFKA_BROKERS,
    #     group_id=f"mt5_worker_{SOURCE_ID}_v2", # Change group name to force a fresh start
    #     value_deserializer=lambda v: json.loads(v.decode('utf-8')),
    #     auto_offset_reset='latest' # Ensure it only listens for NEW commands
    # )
    
    consumer = KafkaConsumer(
        CONTROL_TOPIC,
        bootstrap_servers=KAFKA_BROKERS,
        group_id=f"mt5_worker_v2{int(time.time())}", # Fresh group every start
        value_deserializer=lambda v: json.loads(v.decode('utf-8')),
        auto_offset_reset='latest' # IMPORTANT: Look for commands sent just before Python started
    )

    threading.Thread(target=heartbeat_loop, args=(producer,), daemon=True).start()
    threading.Thread(target=stream_loop, args=(producer,), daemon=True).start()

    try:
        for message in consumer:
            cmd = message.value
            command_type = cmd.get("command")
            
            if command_type == "SYNC":
                handle_sync(cmd, producer)
            elif command_type == "HEAL":
                handle_heal(cmd, producer)
    except Exception as e:
        print(f"Main Consumer Error: {e}")
    finally:
        update_health_status(False)
        mt5.shutdown()

if __name__ == "__main__":
    main()