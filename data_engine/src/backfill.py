

def general_backfill(producer, asset_list, start_ny_dt, end_ny_dt):
    
    """
    Backfills data for a specific New York time range.
    Example: start_ny_dt = datetime(2025, 12, 18, 17, 0)
    """
    # 1. Get the current offset to translate NY to Broker time
    offset = get_broker_offset()
    ny_tz = pytz.timezone("America/New_York")
    
    print(f"🛠️ [BACKFILL] Starting range: {start_ny_dt} to {end_ny_dt} NY")

    for sys_sym in asset_list:
        meta = ACTIVE_STREAMS.get(sys_sym.upper())
        broker_sym = meta['mt5_symbol']

        # 2. Convert NY inputs to Broker-compatible Datetimes
        # NY 18:00 -> UTC 23:00 -> Broker 01:00 (if offset is 7200)
        start_utc = ny_tz.localize(start_ny_dt).astimezone(pytz.UTC)
        end_utc = ny_tz.localize(end_ny_dt).astimezone(pytz.UTC)
        
        # MT5 copy_rates_range expects naive datetimes in Broker Time
        broker_start = datetime.fromtimestamp(start_utc.timestamp() + offset)
        broker_end = datetime.fromtimestamp(end_utc.timestamp() + offset)

        rates = mt5.copy_rates_range(broker_sym, mt5.TIMEFRAME_M15, broker_start, broker_end)

        if rates is not None and len(rates) > 0:
            for rate in rates:
                # 3. Convert Broker time back to UTC Milliseconds for Postgres
                ts_ms = to_utc_ms(rate['time']) 
                
                payload = {
                    "asset_id": f"assets:{sys_sym.upper()}:{SOURCE_ID}",
                    "ts": ts_ms,
                    "open": float(rate['open']),
                    "high": float(rate['high']),
                    "low": float(rate['low']),
                    "close": float(rate['close']),
                    "volume": float(rate['tick_volume']),
                    "source": SOURCE_ID 
                }
                producer.send(f"{sys_sym.lower()}_market_data", value=payload)
            
            print(f"✅ [SUCCESS] {sys_sym}: Sent {len(rates)} bars to ingestion.")
        else:
            print(f"❌ [EMPTY] {sys_sym}: No data found for this range in MT5.")
            
    producer.flush()
    
    
start = datetime(2025, 12, 18, 14, 0)
end = datetime(2025, 12, 18, 16, 0)
general_backfill(producer, ["UK100", "US30"], start, end)