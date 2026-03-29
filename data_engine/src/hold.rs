// Place this near your other lazy_static blocks
lazy_static::lazy_static! {
    static ref BACKFILL_STATE: DashMap<RecordId, (db::MarketData, u64, u64)> = DashMap::new();
}

// ... inside your impl DataIngestionExt for DataService ...

async fn process_tick_backfill_header(
    &self,
    socket: &mut TcpStream,
    context: &IngestionContext,
    dbs: &Arc<AppDatabases>,
) -> db::AppResult<()> {
    let mut buf = [0u8; 64];
    socket.read_exact(&mut buf).await?;

    if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
        let asset_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
        
        let bucket_ms = 300_000; // 5 Minutes
        let bar_start_ts = (tick.time_msc / bucket_ms) * bucket_ms;

        // 1. Get or Create the current aggregation state for this asset
        let mut entry = BACKFILL_STATE.entry(asset_id.clone()).or_insert_with(|| {
            let dt = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
            (
                db::MarketData {
                    asset_id: asset_id.clone(),
                    time: dt,
                    open: tick.last, high: tick.last, low: tick.last, close: tick.last,
                    volume: tick.volume as f64, // Initial official volume
                    buy_volume: 0.0, sell_volume: 0.0,
                },
                0, 0 // Buy count, Sell count
            )
        });

        let (ref mut bar, ref mut buy_count, ref mut sell_count) = *entry;
        let current_bar_ts = time_utils::utc_to_ts_ms(&bar.time);

        // 2. Check for Window Change (Bar is finished)
        if bar_start_ts > current_bar_ts {
            // Calculate final volumes using the Buyer/Seller ratio
            let mut finished_bar = bar.clone();
            let total_ticks = (*buy_count + *sell_count) as f64;
            
            if total_ticks > 0.0 {
                let ratio = *buy_count as f64 / total_ticks;
                finished_bar.buy_volume = finished_bar.volume * ratio;
                finished_bar.sell_volume = finished_bar.volume - finished_bar.buy_volume;
            }

            // 3. Send to your existing batch ingester
            let dbs_disk = Arc::clone(&dbs);
            let service_handle = self.clone();
            tokio::spawn(async move {
                let _ = service_handle.ingest_market_data(&dbs_disk.disk, vec![finished_bar]).await;
            });

            // 4. Reset state for the new window
            bar.time = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
            bar.open = tick.last; bar.high = tick.last; bar.low = tick.last; bar.close = tick.last;
            bar.volume = tick.volume as f64;
            *buy_count = 0;
            *sell_count = 0;
        } else {
            // 5. Still in the same window, just update OHLC and counts
            bar.high = bar.high.max(tick.last);
            bar.low = bar.low.min(tick.last);
            bar.close = tick.last;
            bar.volume = tick.volume as f64; // Keep updating to latest official volume

            if tick.flags & 32 != 0 { *buy_count += 1; }      // TICK_FLAG_BUY
            else if tick.flags & 64 != 0 { *sell_count += 1; } // TICK_FLAG_SELL
        }
    }
    Ok(())
}


-- 1. Update Tick Table to handle Price-Level context
DEFINE TABLE tick SCHEMAFULL CHANGEFEED 1h;
DEFINE FIELD asset_id  ON tick TYPE record<assets> ASSERT $value != NONE;
DEFINE FIELD bid       ON tick TYPE float;
DEFINE FIELD ask       ON tick TYPE float;
DEFINE FIELD price     ON tick TYPE float; -- The price this tick occurred at
DEFINE FIELD volume    ON tick TYPE float; -- The CURRENT total volume of the bar
DEFINE FIELD side      ON tick TYPE string; -- "Buy", "Sell", or "Neutral"
DEFINE FIELD time      ON tick TYPE datetime;

-- 2. Updated Aggregation Event (The Heart of the Live Data)
DEFINE EVENT aggregate_to_5m ON TABLE tick WHEN $event = "CREATE" THEN {
    LET $time = time::floor($after.time, 5m);
    LET $symbol = $after.asset_id.symbol;
    LET $id = type::record('market_data', [$symbol, $time]);
    LET $price_key = type::string($after.price); -- Use price as the object key

    -- 1. Initialize the bar if it's the first tick of the 5m window
    INSERT IGNORE INTO market_data {
        id: $id,
        asset_id: $after.asset_id,
        time: $time,
        open: $after.price,
        high: $after.price,
        low: $after.price,
        close: $after.price,
        volume: $after.volume,
        buy_volume: 0.0,
        sell_volume: 0.0,
        levels: {}
    };

    -- 2. Determine incremental buy/sell (Since Live volume is cumulative, we calculate diff)
    -- For simplicity in live streaming, we track the global volume and side.
    UPDATE $id SET
        high = math::max([high, $after.price]),
        low = math::min([low, $after.price]),
        close = $after.price,
        
        -- Update Global Volumes
        buy_volume += IF $after.side = "Buy" THEN 1.0 ELSE 0.0 END, 
        sell_volume += IF $after.side = "Sell" THEN 1.0 ELSE 0.0 END,
        
        -- Update Total Bar Volume (Anchoring to the latest broker report)
        volume = $after.volume,

        -- Update the Footprint (Price Levels)
        -- We increment the [Buy, Sell] array at the specific price key
        levels.$price_key = IF levels.$price_key = NONE 
            THEN (IF $after.side = "Buy" THEN [1.0, 0.0] ELSE [0.0, 1.0] END)
            ELSE [
                levels.$price_key[0] + (IF $after.side = "Buy" THEN 1.0 ELSE 0.0 END),
                levels.$price_key[1] + (IF $after.side = "Sell" THEN 1.0 ELSE 0.0 END)
            ] END;
};



dbs.mem
    .query("UPSERT $id SET 
        asset_id = $asset, 
        last_bid = $bid, 
        last_ask = $ask,
        last_price = $price, -- Add this to your monitor table!
        volume = $vol, 
        time = $time")
    .bind(("id", monitor_id))
    .bind(("asset", update.asset_id))
    .bind(("bid", update.last_bid))
    .bind(("ask", update.last_ask))
    .bind(("price", if update.last > 0.0 { update.last } else { update.last_bid }))
    .bind(("vol", update.volume))
    .bind(("time", datetime))
    .await?
    .check()?;


    pub async fn flush_all_backfill_batches(&self, dbs: &Arc<AppDatabases>) {
    for mut entry in BACKFILL_STATE.iter_mut() {
        let (bar, buy_cnt, sell_cnt, batch) = entry.value_mut();
        
        // 1. Finalize the very last active bar
        let mut last_bar = bar.clone();
        // ... apply scaling logic here ...
        batch.push(last_bar);

        // 2. Flush the entire batch
        if !batch.is_empty() {
            let to_send = std::mem::take(batch);
            let _ = self.ingest_market_data(&dbs.disk, to_send).await;
        }
    }
    BACKFILL_STATE.clear();
}


async fn process_tick_backfill_header(
    &self,
    socket: &mut TcpStream,
    context: &IngestionContext,
    dbs: &Arc<AppDatabases>,
) -> db::AppResult<()> {
    let mut buf = [0u8; 64];
    socket.read_exact(&mut buf).await?;

    if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
        let asset_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
        let price = if tick.last > 0.0 { tick.last } else { tick.bid };
        let price_key = format!("{:.5}", price);

        let broker_offset_ms = self.get_broker_offset() * 1000;
        let utc_time_msc = tick.time_msc - broker_offset_ms;
        let bucket_ms = 300_000; 
        let bar_start_ts = (utc_time_msc / bucket_ms) * bucket_ms;

        let mut entry = BACKFILL_STATE.entry(asset_id.clone()).or_insert_with(|| {
            let dt = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
            (
                db::MarketData {
                    asset_id: asset_id.clone(),
                    time: dt,
                    open: price, high: price, low: price, close: price,
                    volume: tick.volume as f64,
                    buy_volume: 0.0, sell_volume: 0.0,
                    levels: BTreeMap::new(),
                },
                0, 0,
                Vec::with_capacity(100)
            )
        });

        let (ref mut bar, ref mut total_buy_count, ref mut total_sell_count, ref mut batch) = *entry;
        let current_bar_ts = bar.time.timestamp_millis();

        // --- WINDOW CHANGE: BAR IS FINISHED ---
        if bar_start_ts > current_bar_ts {
            let mut finished_bar = bar.clone();
            
            // 1. Calculate final volumes/deltas
            let total_ticks = (*total_buy_count + *total_sell_count) as f64;
            if total_ticks > 0.0 {
                let scale_factor = finished_bar.volume / total_ticks;
                finished_bar.buy_volume = *total_buy_count as f64 * scale_factor;
                finished_bar.sell_volume = *total_sell_count as f64 * scale_factor;
                for v in finished_bar.levels.values_mut() { v.0 *= scale_factor; v.1 *= scale_factor; }
            }

            // 2. Routing Logic (Like the old way)
            let now_utc = Utc::now().timestamp_millis();
            let is_recent = (now_utc - current_bar_ts).abs() < (10 * 60 * 1000); // Within 10 mins

            if is_recent {
                let dbs_mem = Arc::clone(dbs);
                let monitor_data = finished_bar.clone();
                let monitor_id = db::make_composite_id("monitor", &[&tick.asset, &context.0.data_source_id]);
                
                // SEED the monitor so live ticks have a 'Previous State'
                tokio::spawn(async move {
                    let _ = dbs_mem.mem.query("UPSERT $id SET 
                       asset_id = $asset, last_bid = $close, volume = $vol, time = $time")
                        .bind(("id", monitor_id))
                        .bind(("asset", monitor_data.asset))
                        .bind(("close", monitor_data.close))
                        .bind(("vol", monitor_data.volume))
                        .bind(("time", monitor_data.time))
                        .await;
                });
            }

            // 3. Disk Batch Management
            batch.push(finished_bar);
            if batch.len() >= 100 {
                let to_send = std::mem::replace(batch, Vec::with_capacity(100));
                let dbs_disk = Arc::clone(&dbs);
                let service_handle = self.clone();
                tokio::spawn(async move {
                    let _ = service_handle.ingest_market_data(&dbs_disk.disk, to_send).await;
                });
            }

            // Reset for New Window
            bar.time = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
            bar.open = price; bar.high = price; bar.low = price; bar.close = price;
            bar.volume = tick.volume as f64;
            bar.levels.clear();
            *total_buy_count = 0; *total_sell_count = 0;

        } else {
            // --- UPDATE ONGOING BAR ---
            bar.high = bar.high.max(price);
            bar.low = bar.low.min(price);
            bar.close = price;
            bar.volume = tick.volume as f64;

            // Manual Count Logic
            let is_buy = if tick.last > 0.0 { tick.last >= tick.ask } else { true };
            let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
            if is_buy { level.0 += 1.0; *total_buy_count += 1; } 
            else { level.1 += 1.0; *total_sell_count += 1; }
        }
    }
    Ok(())
}


async fn process_tick_backfill_header(
    &self,
    socket: &mut TcpStream,
    context: &IngestionContext,
    dbs: &Arc<AppDatabases>,
) -> db::AppResult<()> {
    let mut buf = [0u8; 64];
    socket.read_exact(&mut buf).await?;

    if let Ok(tick) = bytemuck::try_from_bytes::<dm::MultiplexedTick>(&buf) {
        let asset_id = self.get_asset_record_id(&tick.asset, &context.0.data_source_id);
        let price = if tick.last > 0.0 { tick.last } else { tick.bid };
        let price_key = format!("{:.5}", price);

        let broker_offset_ms = self.get_broker_offset() * 1000;
        let utc_time_msc = tick.time_msc - broker_offset_ms;
        let bucket_ms = 300_000; 
        let bar_start_ts = (utc_time_msc / bucket_ms) * bucket_ms;

        // CRITICAL: Ensure this matches your DashMap definition
        let mut entry = BACKFILL_STATE.entry(asset_id.clone()).or_insert_with(|| {
            let dt = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
            (
                db::MarketData {
                    asset_id: asset_id.clone(),
                    time: dt,
                    open: price, high: price, low: price, close: price,
                    volume: tick.volume as f64,
                    buy_volume: 0.0, sell_volume: 0.0,
                    levels: BTreeMap::new(),
                },
                0, 0,
                Vec::with_capacity(100) // The batch vec
            )
        });

        let (ref mut bar, ref mut total_buy_count, ref mut total_sell_count, ref mut batch) = *entry;
        let current_bar_ts = bar.time.timestamp_millis();

        if bar_start_ts > current_bar_ts {
            let mut finished_bar = bar.clone();
            
            // Fix Vol: Using our manual counts
            let total_ticks = (*total_buy_count + *total_sell_count) as f64;
            if total_ticks > 0.0 {
                let scale_factor = finished_bar.volume / total_ticks;
                finished_bar.buy_volume = *total_buy_count as f64 * scale_factor;
                finished_bar.sell_volume = *total_sell_count as f64 * scale_factor;
                for v in finished_bar.levels.values_mut() { v.0 *= scale_factor; v.1 *= scale_factor; }
            }

            // Ingest logic: Batch to Disk
            batch.push(finished_bar);
            if batch.len() >= 100 {
                let to_send = std::mem::replace(batch, Vec::with_capacity(100));
                let dbs_disk = Arc::clone(&dbs);
                let service_handle = self.clone();
                tokio::spawn(async move {
                    let _ = service_handle.ingest_market_data(&dbs_disk.disk, to_send).await;
                });
            }

            // Reset for New Window
            bar.time = time_utils::ts_to_utc_datetime(bar_start_ts).map(Datetime::from).unwrap_or_default();
            bar.open = price; bar.high = price; bar.low = price; bar.close = price;
            bar.volume = tick.volume as f64;
            bar.levels.clear();
            *total_buy_count = 0; *total_sell_count = 0;

        } else {
            bar.high = bar.high.max(price);
            bar.low = bar.low.min(price);
            bar.close = price;
            bar.volume = tick.volume as f64;

            // FIX: Don't rely on flags, use price vs ask
            let is_buy = if tick.last > 0.0 { tick.last >= tick.ask } else { true };
            let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
            if is_buy { 
                level.0 += 1.0; 
                *total_buy_count += 1; 
            } else { 
                level.1 += 1.0; 
                *total_sell_count += 1; 
            }
        }
    }
    Ok(())
}

// --- UPDATE ONGOING BAR ---
bar.high = bar.high.max(price);
bar.low = bar.low.min(price);
bar.close = price;
bar.volume = tick.volume as f64;

// 1. Determine the "Active Price" for side calculation
let active_price = if tick.last > 0.0 { tick.last } else { tick.bid };

// 2. Deterministic Side Logic
// If price is >= Ask, it's a Buy. 
// If price is <= Bid, it's a Sell.
// If it's in between, we check which one it's closer to.
let is_buy = if active_price >= tick.ask {
    true
} else if active_price <= tick.bid {
    false
} else {
    // Price is inside the spread (rare in backfill, but possible)
    // Compare distance to Bid vs distance to Ask
    (active_price - tick.bid) > (tick.ask - active_price)
};

// 3. Update the Level and Global Counts
let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
if is_buy { 
    level.0 += 1.0; 
    *total_buy_count += 1; 
} else { 
    level.1 += 1.0; 
    *total_sell_count += 1; 
}

// 1. Get the previous price from the bar's current 'close' (which was the last tick's price)
let prev_price = bar.close; 

// 2. Logic: If price went up, it's a buy. If down, it's a sell.
let is_buy = if price > prev_price {
    true
} else if price < prev_price {
    false
} else {
    // If price didn't change (Flat Tick), fall back to Quote Comparison
    // This prevents the "All Zero" problem
    price >= tick.ask 
};

// 3. Update the counts
let level = bar.levels.entry(price_key).or_insert((0.0, 0.0));
if is_buy { 
    level.0 += 1.0; 
    *total_buy_count += 1; 
} else { 
    level.1 += 1.0; 
    *total_sell_count += 1; 
}

// 4. Update close for the NEXT tick's comparison
bar.close = price;


DEFINE EVENT on_tick_received ON TABLE monitor WHEN $event = "UPDATE" THEN {
    -- 1. Calculate the Volume Delta (Actual Broker Volume change)
    -- This ensures we use the broker's scale, not just a 1.0 increment
    LET $vol_delta = math::max([0.0, $after.volume - $before.volume]);
    
    -- 2. Determine Side (Your existing Quote Movement logic)
    LET $bid_diff = $after.last_bid - $before.last_bid;
    LET $ask_diff = $after.last_ask - $before.last_ask;
    
    LET $side = IF $ask_diff > 0 THEN "Buy" 
                ELSE IF $bid_diff < 0 THEN "Sell" 
                ELSE "Neutral" END;

    -- 3. If side is Neutral, we still need to assign volume (Industry standard: use the Spread)
    LET $assigned_side = IF $side != "Neutral" THEN $side 
                         ELSE (IF $after.last_bid >= $after.last_ask THEN "Buy" ELSE "Sell") END;

    -- 4. Atomic Update to MarketData
    LET $bar_id = type::thing("market_data", [$after.asset_id[0], time::floor($after.time, 5m)]);
    LET $price_key = type::string($after.last_bid);

    UPDATE $bar_id SET
        high = math::max([high, $after.last_bid]),
        low = math::min([low, $after.last_bid]),
        close = $after.last_bid,
        volume = $after.volume, -- Keep absolute broker volume

        -- Update Global Buy/Sell using the delta to stay on the same scale
        buy_volume += IF $assigned_side = "Buy" THEN $vol_delta ELSE 0.0 END, 
        sell_volume += IF $assigned_side = "Sell" THEN $vol_delta ELSE 0.0 END,

        -- Update the Footprint Levels
        levels[$price_key] = IF levels[$price_key] = NONE 
            THEN (IF $assigned_side = "Buy" THEN [$vol_delta, 0.0] ELSE [0.0, $vol_delta] END)
            ELSE [
                levels[$price_key][0] + (IF $assigned_side = "Buy" THEN $vol_delta ELSE 0.0 END),
                levels[$price_key][1] + (IF $assigned_side = "Sell" THEN $vol_delta ELSE 0.0 END)
            ] END;
};

DEFINE EVENT on_tick_received ON TABLE monitor WHEN $event = "UPDATE" THEN {
    -- 1. Determine volume change since the last tick
    -- Use math::max to prevent negative spikes during bar resets
    LET $vol_delta = math::max([0.0, $after.volume - $before.volume]);
    
    -- 2. Quote Movement Logic
    LET $bid_diff = $after.last_bid - $before.last_bid;
    LET $ask_diff = $after.last_ask - $before.last_ask;
    
    LET $side = IF $ask_diff > 0 THEN "Buy" 
                ELSE IF $bid_diff < 0 THEN "Sell" 
                ELSE "Neutral" END;

    -- 3. Create the tick with the Broker Delta instead of raw volume
    -- This ensures the downstream aggregator knows exactly how much volume this tick added
    CREATE tick SET
        asset_id = $after.asset_id,
        bid = $after.last_bid,
        ask = $after.last_ask,
        bid_delta = $bid_diff,
        ask_delta = $ask_diff,
        spread = $after.last_ask - $after.last_bid,
        vol_delta = $vol_delta, -- The key for scaling
        total_volume = $after.volume,
        time = $after.time,
        side = $side;
};

DEFINE EVENT aggregate_to_5m ON TABLE tick WHEN $event = "CREATE" THEN {
    LET $time = time::floor($after.time, 5m);
    LET $symbol = $after.asset_id[0]; -- Extracting symbol from the record ID array
    LET $id = type::record('market_data', [$symbol, $time]);
    LET $price_key = <string>$after.bid;
    
    -- Assign a side for "Neutral" ticks (Aggressor Fallback)
    LET $assigned_side = IF $after.side != "Neutral" THEN $after.side 
                         ELSE (IF $after.bid >= $after.ask THEN "Buy" ELSE "Sell") END;

    -- 1. Initialize the Bar if it doesn't exist
    INSERT IGNORE INTO market_data {
        id: $id,
        asset_id: $after.asset_id,
        time: $time,
        open: $after.bid,
        high: $after.bid,
        low: $after.bid,
        close: $after.bid,
        volume: 0.0,
        buy_volume: 0.0,
        sell_volume: 0.0,
        levels: {}
    };

    -- 2. Atomic Update using the Vol Delta
    UPDATE $id SET
        high = math::max([high, $after.bid]),
        low = math::min([low, $after.bid]),
        close = $after.bid,
        volume += $after.vol_delta,

        -- Scale Global Volumes using the broker's delta
        buy_volume += IF $assigned_side = "Buy" THEN $after.vol_delta ELSE 0.0 END, 
        sell_volume += IF $assigned_side = "Sell" THEN $after.vol_delta ELSE 0.0 END,

        -- Update the Footprint Levels
        levels[$price_key] = IF levels[$price_key] = NONE 
            THEN (IF $assigned_side = "Buy" THEN [$after.vol_delta, 0.0] ELSE [0.0, $after.vol_delta] END)
            ELSE [
                levels[$price_key][0] + (IF $assigned_side = "Buy" THEN $after.vol_delta ELSE 0.0 END),
                levels[$price_key][1] + (IF $assigned_side = "Sell" THEN $after.vol_delta ELSE 0.0 END)
            ] END;
};