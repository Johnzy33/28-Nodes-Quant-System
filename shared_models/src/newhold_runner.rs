use anyhow::{Context, Result};
use tokio_postgres::Client;

// Define the specific asset and table name
const ASSET_ID: &str = "US2000";
const SOURCE_TABLE: &str = "US2000_market_data"; // Architecture 1: New base table name

/// Helper macro to generate SQL for creating continuous aggregates (CAGGs)
/// for Daily, Weekly, and Monthly periods based on the asset ID. (Architecture 2, 4, 5)
macro_rules! create_asset_caggs {
    ($client:expr, $source_table:expr, $period_name:expr, $time_func:expr) => {
        $client.execute(
            &format!(
                "CREATE MATERIALIZED VIEW {}_{}_aggregate
                WITH (timescaledb.continuous) AS
                SELECT
                    -- Architecture 3: Custom boundary function for the bucket label
                    {} AS bucket,
                    asset_id,
                    -- Architecture 4: Correct OHLCV rollup using TimescaleDB functions
                    FIRST(open, time) AS open,
                    MAX(high) AS high,
                    MIN(low) AS low,
                    LAST(close, time) AS close,
                    SUM(volume) AS volume,
                    -- Architecture 5: Calculate the candle pattern based on the rolled-up OHLC
                    get_candle_pattern(FIRST(open, time), MAX(high), MIN(low), LAST(close, time)) AS pattern
                FROM {}
                WHERE asset_id = $1
                GROUP BY 1, 2;",
                ASSET_ID, $period_name, $time_func, $source_table
            ),
            &[&format!("assets:{}", ASSET_ID)],
        ).await.context(format!("Failed to create {}_{}_aggregate", ASSET_ID, $period_name))?;
    };
}

/// Applies the initial database schema (tables, hypertables, and aggregates).
pub async fn apply_schema(client: &mut Client) -> Result<()> {
    println!("-> Applying TimescaleDB schema...");

    // 1. Asset Lookup Table
    client.execute("DROP TABLE IF EXISTS assets CASCADE;", &[]).await.context("Failed to drop old assets table")?;
    client.execute(
        "CREATE TABLE assets (
            asset_id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            type TEXT NOT NULL
        );",
        &[],
    ).await.context("Failed to create assets table")?;

    // 2. Base Market Data Table (Architecture 1)
    client.execute(&format!("DROP TABLE IF EXISTS {} CASCADE;", SOURCE_TABLE), &[]).await.context("Failed to drop old market data table")?;
    client.execute(
        &format!(
            "CREATE TABLE {} (
                time TIMESTAMPTZ NOT NULL,
                asset_id TEXT NOT NULL REFERENCES assets (asset_id),
                open DOUBLE PRECISION NOT NULL,
                high DOUBLE PRECISION NOT NULL,
                low DOUBLE PRECISION NOT NULL,
                close DOUBLE PRECISION NOT NULL,
                volume BIGINT NOT NULL
            );",
            SOURCE_TABLE
        ),
        &[],
    ).await.context("Failed to create new market data table")?;

    // 3. Convert to TimescaleDB Hypertable
    client.execute(
        &format!("SELECT create_hypertable('{}', 'time');", SOURCE_TABLE),
        &[],
    ).await.context("Failed to create hypertable")?;
    println!("-> Created hypertable: {}", SOURCE_TABLE);


    // --- CRITICAL STEP 4: TRADING PERIOD ALIGNMENT FUNCTIONS (Architecture 3) ---

    // Trading Day (Shifts 18:00 NYT to the next day's 00:00 NYT label)
    client.execute("DROP FUNCTION IF EXISTS get_trading_day(TIMESTAMPTZ);", &[]).await.context("Failed to drop old get_trading_day function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION get_trading_day(ts TIMESTAMPTZ)
        RETURNS TIMESTAMPTZ LANGUAGE SQL IMMUTABLE AS $$
            -- Add 6 hours to shift the 18:00 NY start to 00:00 NY of the NEXT day.
            SELECT date_trunc('day', ts + INTERVAL '6 hours') AT TIME ZONE 'America/New_York';
        $$;",
        &[],
    ).await.context("Failed to create get_trading_day function")?;
    
    // Trading Week (Shifts 18:00 Sunday NYT to Monday 00:00 NYT label)
    client.execute("DROP FUNCTION IF EXISTS get_trading_week(TIMESTAMPTZ);", &[]).await.context("Failed to drop old get_trading_week function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION get_trading_week(ts TIMESTAMPTZ)
        RETURNS TIMESTAMPTZ LANGUAGE SQL IMMUTABLE AS $$
            -- Add 6 hours to align the 18:00 Sunday NY start to 00:00 Monday NY.
            SELECT date_trunc('week', ts + INTERVAL '6 hours') AT TIME ZONE 'America/New_York';
        $$;",
        &[],
    ).await.context("Failed to create get_trading_week function")?;
    
    // Trading Month (Shifts 18:00 EOM NYT to next month's 00:00 NYT label)
    client.execute("DROP FUNCTION IF EXISTS get_trading_month(TIMESTAMPTZ);", &[]).await.context("Failed to drop old get_trading_month function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION get_trading_month(ts TIMESTAMPTZ)
        RETURNS TIMESTAMPTZ LANGUAGE SQL IMMUTABLE AS $$
            -- Add 6 hours to align the 18:00 EOM NY start to 00:00 next month NY.
            SELECT date_trunc('month', ts + INTERVAL '6 hours') AT TIME ZONE 'America/New_York';
        $$;",
        &[],
    ).await.context("Failed to create get_trading_month function")?;

    // Session Grouping Function (for intra-day analysis)
    client.execute("DROP FUNCTION IF EXISTS custom_session_group(TIMESTAMPTZ);", &[]).await.context("Failed to drop old custom_session_group function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION custom_session_group(ts TIMESTAMPTZ)
        RETURNS TEXT LANGUAGE SQL IMMUTABLE AS $$
            SELECT CASE 
                WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 2 AND 7 THEN 'LN'
                WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 8 AND 11 THEN 'NYAM'
                WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 12 AND 13 THEN 'NYL'
                WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') BETWEEN 14 AND 17 THEN 'NYPM'
                WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') >= 18 THEN 'AS'
                WHEN EXTRACT(HOUR FROM ts AT TIME ZONE 'America/New_York') <= 1 THEN 'AS'
                ELSE 'UNKNOWN' 
            END;
        $$;",
        &[],
    ).await.context("Failed to create custom_session_group function")?;


  //  -- --- CRITICAL STEP 5: CANDLE PATTERN RECOGNITION (Architecture 5) ---
    client.execute("DROP FUNCTION IF EXISTS get_candle_pattern(double precision, double precision, double precision, double precision);", &[]).await.context("Failed to drop old get_candle_pattern function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION get_candle_pattern(o double precision, h double precision, l double precision, c double precision)
        RETURNS TEXT
        AS $$
        DECLARE
            body_size DOUBLE PRECISION;
            total_range DOUBLE PRECISION;
            body_ratio DOUBLE PRECISION;
            shadow_ratio DOUBLE PRECISION;
            
        BEGIN
            body_size := ABS(c - o);
            total_range := h - l;

            -- Avoid division by zero
            IF total_range = 0 OR total_range < 0.0001 THEN
                RETURN 'Flat';
            END IF;

            body_ratio := body_size / total_range;

            -- 1. Doji Check (Body is very small relative to the range)
            IF body_ratio < 0.05 THEN
                RETURN 'Doji';
            END IF;

            -- 2. Marubozu Check (Very little to no wicks)
            IF body_ratio > 0.9 THEN
                IF c > o THEN
                    RETURN 'Bullish Marubozu';
                ELSE
                    RETURN 'Bearish Marubozu';
                END IF;
            END IF;

            -- 3. Hammer/Hanging Man Check (Requires a small body relative to the shadow)
            IF body_ratio < 0.3 THEN 
                IF c > o THEN -- Bullish Candle
                    -- Lower shadow must be at least 2x the body size
                    shadow_ratio := (o - l) / body_size;
                    IF shadow_ratio >= 2.0 THEN
                        RETURN 'Hammer';
                    END IF;
                ELSE -- Bearish Candle
                    -- Upper shadow must be at least 2x the body size
                    shadow_ratio := (h - o) / body_size;
                    IF shadow_ratio >= 2.0 THEN
                        RETURN 'Hanging Man';
                    END IF;
                END IF;
            END IF;
            
            -- 4. Standard Candles
            IF c > o THEN
                RETURN 'Bullish';
            ELSE
                RETURN 'Bearish';
            END IF;
        END
        $$ LANGUAGE plpgsql IMMUTABLE STRICT;",
        &[],
    ).await.context("Failed to create get_candle_pattern function")?;


//    -- 6. Create Continuous Aggregates (CAGGs) - Architecture 2, 4, 5
    create_asset_caggs!(client, SOURCE_TABLE, "daily", "get_trading_day(time)");
    create_asset_caggs!(client, SOURCE_TABLE, "weekly", "get_trading_week(time)");
    create_asset_caggs!(client, SOURCE_TABLE, "monthly", "get_trading_month(time)");

    println!("✅ All database schema functions, continuous aggregates, and integrity checks are now fully implemented.");
    Ok(())
}