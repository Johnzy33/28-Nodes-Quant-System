use anyhow::{Context, Result};
use tokio_postgres::{Client, NoTls};

// --- DATABASE CONFIGURATION AND CONNECTION POOL (omitted for brevity) ---
fn get_postgres_config() -> Result<tokio_postgres::Config> {
    let db_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL environment variable not set")?;
    let mut config = tokio_postgres::Config::new();
    config.options(&db_url);
    if let Some(user) = db_url.split('@').next() {
        if let Some((u, p)) = user.split("//").last().and_then(|s| s.split_once(':')) {
            config.user(u).password(p);
        }
    }
    config.dbname("market_data"); 
    Ok(config)
}
pub async fn setup_database_pool() -> Result<Client> {
    let config = get_postgres_config()?;
    println!("-> Connecting to Postgres database...");
    let (client, connection) = config.connect(NoTls).await
        .context("Failed to connect to PostgreSQL database")?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("database connection error: {}", e);
        }
    });
    println!("✅ Database connection established.");
    Ok(client)
}

// --- CORE SCHEMA AND AGGREGATION LOGIC ---

/// Applies the initial database schema (tables, hypertables, and aggregates).
pub async fn apply_schema(client: &mut Client) -> Result<()> {
    println!("-> Applying TimescaleDB schema...");

    // 1-3. Base table setup (assets, market_data_1hr) remains the same
    client.execute(
        "CREATE TABLE IF NOT EXISTS assets (id TEXT PRIMARY KEY, name TEXT NOT NULL);",
        &[],
    ).await.context("Failed to create assets table")?;
    client.execute(
        "CREATE TABLE IF NOT EXISTS market_data_1hr (
            time TIMESTAMPTZ NOT NULL, asset_id TEXT NOT NULL, open DOUBLE PRECISION NOT NULL, 
            high DOUBLE PRECISION NOT NULL, low DOUBLE PRECISION NOT NULL, close DOUBLE PRECISION NOT NULL, 
            volume DOUBLE PRECISION NOT NULL, PRIMARY KEY (time, asset_id), 
            CONSTRAINT fk_asset FOREIGN KEY (asset_id) REFERENCES assets (id)
        );",
        &[],
    ).await.context("Failed to create market_data_1hr table")?;
    client.execute("SELECT create_hypertable('market_data_1hr', 'time', if_not_exists => TRUE);", &[]).await.context("Failed to convert market_data_1hr to hypertable")?;
    client.execute("CREATE INDEX IF NOT EXISTS idx_market_data_asset_id ON market_data_1hr (asset_id);", &[]).await.context("Failed to create index on market_data_1hr (asset_id)")?;


    // --- CRITICAL STEP 4: TRADING PERIOD ALIGNMENT & PATTERN LOGIC FUNCTIONS ---

    // 4a. Trading Period Normalization Functions (Aligns day, week, month to 18:00 NY start)
    // Trading Day
    client.execute("DROP FUNCTION IF EXISTS get_trading_day(TIMESTAMPTZ);", &[]).await.context("Failed to drop old get_trading_day function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION get_trading_day(ts TIMESTAMPTZ)
        RETURNS TIMESTAMPTZ LANGUAGE SQL IMMUTABLE AS $$
            -- Shifts time forward 6 hours (18:00 NY boundary) before truncation to get the correct trading day label.
            SELECT date_trunc('day', ts AT TIME ZONE 'America/New_York' + INTERVAL '6 hours') AT TIME ZONE 'America/New_York';
        $$;",
        &[],
    ).await.context("Failed to create get_trading_day function")?;
    
    // Trading Week
    client.execute("DROP FUNCTION IF EXISTS get_trading_week(TIMESTAMPTZ);", &[]).await.context("Failed to drop old get_trading_week function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION get_trading_week(ts TIMESTAMPTZ)
        RETURNS TIMESTAMPTZ LANGUAGE SQL IMMUTABLE AS $$
            -- Shifts time forward 6 hours (18:00 NY boundary) before truncation to get the correct trading week label.
            SELECT date_trunc('week', ts AT TIME ZONE 'America/New_York' + INTERVAL '6 hours') AT TIME ZONE 'America/New_York';
        $$;",
        &[],
    ).await.context("Failed to create get_trading_week function")?;
    
    // Trading Month
    client.execute("DROP FUNCTION IF EXISTS get_trading_month(TIMESTAMPTZ);", &[]).await.context("Failed to drop old get_trading_month function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION get_trading_month(ts TIMESTAMPTZ)
        RETURNS TIMESTAMPTZ LANGUAGE SQL IMMUTABLE AS $$
            -- Shifts time forward 6 hours (18:00 NY boundary) before truncation to get the correct trading month label.
            SELECT date_trunc('month', ts AT TIME ZONE 'America/New_York' + INTERVAL '6 hours') AT TIME ZONE 'America/New_York';
        $$;",
        &[],
    ).await.context("Failed to create get_trading_month function")?;


    // 4b. Session Grouping Function (unchanged)
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

    // 4c. Pattern Classification Function (PL/pgSQL translation of the Rust logic)
    client.execute("DROP FUNCTION IF EXISTS calculate_candle_pattern(double precision, double precision, double precision, double precision);", &[]).await.context("Failed to drop old calculate_candle_pattern function")?;
    client.execute(
        "CREATE OR REPLACE FUNCTION calculate_candle_pattern(
            open_val double precision, high_val double precision, 
            low_val double precision, close_val double precision
        )
        RETURNS TEXT LANGUAGE plpgsql IMMUTABLE AS $$
        DECLARE
            range DOUBLE PRECISION;
            body_size DOUBLE PRECISION;
            upper_wick DOUBLE PRECISION;
            lower_wick DOUBLE PRECISION;
            upper_threshold DOUBLE PRECISION;
            lower_threshold DOUBLE PRECISION;
            body_range_ratio DOUBLE PRECISION;
        BEGIN
            -- Calculate key metrics
            range := high_val - low_val;
            
            -- Handle zero range to prevent division by zero and return 'Doji'
            IF range = 0 THEN
                RETURN 'Doji';
            END IF;

            body_size := abs(close_val - open_val);
            upper_wick := high_val - greatest(open_val, close_val);
            lower_wick := least(open_val, close_val) - low_val;
            body_range_ratio := body_size / range;

            -- Thresholds (Simplified heuristics based on common patterns)
            upper_threshold := 0.25; -- e.g., upper wick is less than 25% of body
            lower_threshold := 0.25; -- e.g., lower wick is less than 25% of body
            
            -- 1. Large Body Patterns (Engulfing / Marubozu-like)
            IF body_range_ratio >= 0.7 THEN
                IF close_val > open_val THEN
                    RETURN 'Bullish_Marubozu'; -- Strongest bullish signal
                ELSE
                    RETURN 'Bearish_Marubozu'; -- Strongest bearish signal
                END IF;
            END IF;

            -- 2. Doji/Spinning Top (Indecision)
            IF body_range_ratio < 0.1 THEN
                RETURN 'Doji';
            END IF;

            -- 3. Hammer / Shooting Star (Reversal)
            IF body_range_ratio <= 0.3 THEN
                -- Hammer (Bullish Reversal): Small body, long lower wick, small upper wick.
                IF close_val > open_val AND lower_wick >= 2 * body_size AND upper_wick < body_size THEN
                    RETURN 'Hammer';
                -- Shooting Star (Bearish Reversal): Small body, long upper wick, small lower wick.
                ELSIF open_val > close_val AND upper_wick >= 2 * body_size AND lower_wick < body_size THEN
                    RETURN 'Shooting_Star';
                END IF;
            END IF;

            -- 4. Regular Candle Classification
            IF close_val > open_val THEN
                RETURN 'Bullish';
            ELSE
                RETURN 'Bearish';
            END IF;

            -- Fallback (Should not be reached)
            RETURN 'Neutral';
        END;
        $$;",
        &[],
    ).await.context("Failed to create calculate_candle_pattern function")?;

    
    // --- STEP 5: CREATING TABLES AND CAGGS FOR ALL PERIODS (Daily, Weekly, Monthly) ---
    
    // Helper macro for repeating table and CAGG creation
    macro_rules! create_agg_pair {
        ($period:expr, $table:expr, $cagg:expr, $time_func:expr, $group_by_session:expr) => {
            // 5a. Create the physical TABLE
            let group_cols = if $group_by_session { 
                "session_name TEXT NOT NULL," 
            } else { "" };
            let group_pk = if $group_by_session { 
                "time, asset_id, session_name" 
            } else { "time, asset_id" };
            
            client.execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        time TIMESTAMPTZ NOT NULL,
                        asset_id TEXT NOT NULL,
                        {}
                        open DOUBLE PRECISION NOT NULL,
                        high DOUBLE PRECISION NOT NULL,
                        low DOUBLE PRECISION NOT NULL,
                        close DOUBLE PRECISION NOT NULL,
                        volume DOUBLE PRECISION NOT NULL,
                        high_time TIMESTAMPTZ,              
                        low_time TIMESTAMPTZ,               
                        candle_pattern TEXT NOT NULL,       -- Now populated by the CAGG
                        
                        PRIMARY KEY ({}),
                        CONSTRAINT fk_asset_{}_session
                            FOREIGN KEY (asset_id)
                            REFERENCES assets (id)
                    );", 
                    $table, group_cols, group_pk, $table
                ),
                &[],
            ).await.context(format!("Failed to create {} table", $table))?;


            // 5b. Create the Continuous Aggregate VIEW
            client.execute(&format!("DROP MATERIALIZED VIEW IF EXISTS {} CASCADE;", $cagg), &[]).await
                .context(format!("Failed to drop old {} CAGG", $cagg))?;
            
            let group_by_select = if $group_by_session {
                "custom_session_group(time) AS session_name,"
            } else { "" };
            let group_by_agg = if $group_by_session {
                ", custom_session_group(time)"
            } else { "" };
            
            client.execute(
                &format!(
                    "CREATE MATERIALIZED VIEW {}
                    WITH (timescaledb.continuous, timescaledb.target_table = '{}')
                    AS
                        SELECT
                            {}(time) AS time, 
                            asset_id,
                            {}
                            first(open, time) AS open,
                            max(high) AS high,
                            min(low) AS low,
                            last(close, time) AS close,
                            sum(volume) AS volume,
                            (array_agg(time ORDER BY high DESC, time ASC))[1] AS high_time,
                            (array_agg(time ORDER BY low ASC, time ASC))[1] AS low_time,
                            -- FIX: Calculate pattern directly in the CAGG
                            calculate_candle_pattern(first(open, time), max(high), min(low), last(close, time)) AS candle_pattern
                        FROM market_data_1hr
                        GROUP BY
                            {}(time),
                            asset_id
                            {};", 
                    $cagg, $table, $time_func, group_by_select, $time_func, group_by_agg
                ),
                &[],
            ).await.context(format!("Failed to create {} CAGG definition", $cagg))?;


            // 5c. Set the policy
            client.execute(
                &format!(
                    "SELECT add_continuous_aggregate_policy('{}',
                        start_offset => INTERVAL '1 week',
                        end_offset => INTERVAL '1 hour',
                        schedule_interval => INTERVAL '15 minutes',
                        if_not_exists => TRUE);",
                    $cagg
                ),
                &[],
            ).await.context(format!("Failed to add CAGG policy for {}", $cagg))?;

            println!("   - Created {} and {}", $table, $cagg);
        };
    }

    // SESSION AGGREGATION (Grouped by Day AND Session)
    create_agg_pair!("Session", "session_candles", "session_aggregate_source", "get_trading_day", true);

    // DAILY AGGREGATION (Grouped by Day only)
    create_agg_pair!("Daily", "daily_candles", "daily_aggregate_source", "get_trading_day", false);

    // WEEKLY AGGREGATION (Grouped by Week)
    create_agg_pair!("Weekly", "weekly_candles", "weekly_aggregate_source", "get_trading_week", false);

    // MONTHLY AGGREGATION (Grouped by Month)
    create_agg_pair!("Monthly", "monthly_candles", "monthly_aggregate_source", "get_trading_month", false);


    // --- STEP 6: PREDICTIONS Table (Final Target remains the same) ---
    client.execute(
        "CREATE TABLE IF NOT EXISTS predictions (
            time TIMESTAMPTZ NOT NULL, asset_id TEXT NOT NULL, session_name TEXT NOT NULL, 
            prediction_score DOUBLE PRECISION NOT NULL, signal TEXT,
            PRIMARY KEY (time, asset_id, session_name),
            CONSTRAINT fk_asset_prediction FOREIGN KEY (asset_id) REFERENCES assets (id)
        );",
        &[],
    ).await.context("Failed to create predictions table")?;
    client.execute("SELECT create_hypertable('predictions', 'time', if_not_exists => TRUE);", &[]).await.context("Failed to convert predictions to hypertable")?;
    client.execute("CREATE INDEX IF NOT EXISTS idx_predictions_asset_id ON predictions (asset_id);", &[]).await.context("Failed to create index on predictions (asset_id)")?;


    println!("✅ All database schema, functions, and continuous aggregates are now created with corrected time alignment and integrated pattern classification.");
    Ok(())
}
