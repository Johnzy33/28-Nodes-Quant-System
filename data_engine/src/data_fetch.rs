use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc, Datelike, Duration, NaiveDate}; 
use sqlx;
use shared_models::candle_pattern::{get_market_classification}; 
use shared_models::signal_type::{MarketType};
use std::collections::HashMap;
use crate::data_service::DataService;
use crate::data_model as dm;
use shared_models as sm;



// Helper enum for generic watermark handling
pub enum WatermarkValue { 
    DateTime(DateTime<Utc>), 
    Date(NaiveDate) 
}

impl From<DateTime<Utc>> for WatermarkValue { 
    fn from(dt: DateTime<Utc>
    ) -> Self {
    Self::DateTime(dt) } 
}

impl From<NaiveDate> for WatermarkValue { 
    fn from(d: NaiveDate
    ) -> Self { Self::Date(d) } 
}



pub fn generate_context<T, R, F>(data: &[T], mut transform: F) -> Vec<R>
where
    F: FnMut(&T, Option<&T>, Option<&T>) -> Option<R>,
{
    let mut results = Vec::new();
    
    // We use indices or windows to avoid cloning the elements inside the loop
    for i in 0..data.len() {
        let current = &data[i];
        let p1 = if i >= 1 { Some(&data[i - 1]) } else { None };
        let p2 = if i >= 2 { Some(&data[i - 2]) } else { None };

        if let Some(res) = transform(current, p1, p2) {
            results.push(res);
        }
    }
    results
}

impl DataService {

    /// Generic High Watermark: Works for any table, column, and return type (Date or DateTime)
    async fn get_hwm<T>(
        &self, 
        table: &str, 
        column: &str, 
        asset_id: &str
    ) -> Result<Option<T>>
    where
        T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
    {
        let query = format!("SELECT MAX({}) FROM {} WHERE asset_id = $1", column, table);
        let result: Option<Option<T>> = sqlx::query_scalar(&query)
            .bind(asset_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow!("HWM Error {}.{}: {}", table, column, e))?;
        Ok(result.flatten())
    }

    /// Unified Incremental Fetcher: Handles the "Buffer" logic for both Date and DateTime types
    async fn fetch_buffered<T, H>(
        &self, 
        query_base: &str, 
        asset_id: &str, 
        hwm: Option<H>, 
        buffer: Duration,
        filter_col: &str, // Explicitly pass the column name (e.g., "month_start")
    ) -> Result<Vec<T>>
    where 
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
        H: Into<WatermarkValue> + Copy,
    {
        let mut final_query = match hwm {
            Some(_) => {
                // Use the provided filter_col instead of guessing
                format!("{} AND {} >= $2 ORDER BY {} ASC", query_base, filter_col, filter_col)
            },
            None => format!("{} ORDER BY 1 ASC", query_base),
        };

        let q = sqlx::query_as::<_, T>(&final_query).bind(asset_id);
        
        let result = match hwm {
            Some(h) => match h.into() {
                WatermarkValue::DateTime(ts) => q.bind(ts - buffer).fetch_all(&self.pool).await,
                WatermarkValue::Date(d) => q.bind(d - Duration::days(buffer.num_days())).fetch_all(&self.pool).await,
            },
            None => q.fetch_all(&self.pool).await,
        };

        result.map_err(|e| anyhow!("Buffered Fetch Failed on {}: {}", filter_col, e))
    }

    pub async fn calculate_session_and_context(
        &self, 
        asset_id: &str
    ) -> Result<(Vec<dm::ClassifiedSession>, Vec<sm::SessionContextData>)> {

        let hwm: Option<DateTime<Utc>> = self.get_hwm("session_context", "session_end_ts", asset_id).await?;
        
        let raw: Vec<dm::RawSessionData> = self.fetch_buffered(
            "SELECT * FROM session_base WHERE asset_id = $1", 
            asset_id, hwm, Duration::hours(1),
            "end_ts"
        ).await?;

        // Map raw data to classified sessions
        let classified: Vec<dm::ClassifiedSession> = raw.into_iter().map(|s| dm::ClassifiedSession {
            session_type: get_market_classification(s.open, s.high, s.low, s.close),
            trading_date: s.trading_date, 
            asset_id: s.asset_id, 
            session_name: s.session_name,
            start_ts: s.start_ts, 
            end_ts: s.end_ts, 
            open: s.open, 
            high: s.high, 
            high_ts: s.high_ts, 
            low: s.low, 
            low_ts: s.low_ts, 
            close: s.close, 
            volume: s.volume, 
            bars: s.bars
        }).collect();

        // OPTIMIZED: Passing &classified as a slice (&[T]) avoids a full heap clone
        let contexts = generate_context(&classified, |curr, p1, p2| {
            p1.map(|prev1| sm::SessionContextData {
                trading_date: curr.trading_date,
                asset_id: curr.asset_id.clone(),
                session_end_ts: curr.end_ts,
                cs_name: Some(curr.session_name.clone()),
                cs_bias: Some(curr.session_type.to_string()),
                ps1_name: Some(prev1.session_name.clone()),
                ps1_bias: Some(prev1.session_type.to_string()),
                ps2_name: p2.map(|p| p.session_name.clone()),
                ps2_bias: p2.map(|p| p.session_type.to_string()),
            })
        });

        let limit = hwm.unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
        
        Ok((
            classified.into_iter().filter(|s| s.end_ts >= limit).collect(),
            contexts.into_iter().filter(|c| c.session_end_ts >= limit).collect()
        ))
    }

    pub async fn calculate_8hr_context_and_blocks(
        &self,
        asset_id: &str,
    ) -> Result<(Vec<dm::Classified8HrBlock>, Vec<sm::EightContextData>)> {
        
        let hwm: Option<DateTime<Utc>> = self.get_hwm("block_base", "end_ts", asset_id).await?;
        
        let raw: Vec<dm::Raw8HrBlock> = self.fetch_buffered(
            "SELECT * FROM eight_hr_base WHERE asset_id = $1",
            asset_id,
            hwm,
            Duration::hours(1),
            "end_ts"

        ).await?;

        if raw.is_empty() { return Ok((Vec::new(), Vec::new())); }

        let classified: Vec<dm::Classified8HrBlock> = raw.into_iter().map(|r| dm::Classified8HrBlock {
            block_type: get_market_classification(r.open, r.high, r.low, r.close),
            trading_date: r.trading_date, 
            asset_id: r.asset_id, 
            block_number: r.block_number,
            start_ts: r.start_ts, 
            end_ts: r.end_ts, 
            open: r.open, 
            high: r.high,
            low: r.low, 
            close: r.close, 
            volume: r.volume, 
            bars: r.bars,
        }).collect();

        // OPTIMIZED: generate_context now borrows 'classified'
        let contexts = generate_context(&classified, |curr, p1, p2| {
            p1.map(|pb1| sm::EightContextData {
                trading_date: curr.trading_date,
                asset_id: curr.asset_id.clone(),
                session_end_ts: curr.end_ts,
                cb_num: Some(curr.block_number),
                cb_bias: Some(curr.block_type.to_string()),
                pb1_num: Some(pb1.block_number),
                pb1_bias: Some(pb1.block_type.to_string()),
                pb2_num: p2.map(|p| p.block_number),
                pb2_bias: p2.map(|p| p.block_type.to_string()),
            })
        });

        let limit = hwm.unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
        
        Ok((
            classified.into_iter().filter(|b| b.end_ts >= limit).collect(),
            contexts.into_iter().filter(|c| c.session_end_ts >= limit).collect()
        ))
    }

    pub async fn daily_views(
        &self,
        asset_id: &str,
    ) -> Result<Vec<dm::ClassifiedDailyView>> {
        let hwm: Option<NaiveDate> = self.get_hwm("daily_views", "trading_date", asset_id).await?;
        
        // Fetch daily_base with 7-day buffer to populate Prior Day and Prior Week high/lows
        let raw_data: Vec<dm::RawDailyData> = self.fetch_buffered(
            "SELECT * FROM daily_base WHERE asset_id = $1",
            asset_id,
            hwm,
            Duration::days(7),
            "trading_date" // Explicitly specify the filter column

        ).await?;

        if raw_data.is_empty() { return Ok(Vec::new()); }

        let mut daily_views = Vec::with_capacity(raw_data.len());
        let mut prior_day_high: Option<f64> = None;
        let mut prior_day_low: Option<f64> = None;
        
        // Tracking for rolling 7-day window
        let mut high_history: Vec<(NaiveDate, f64)> = Vec::new();
        let mut low_history: Vec<(NaiveDate, f64)> = Vec::new();

        for raw in raw_data {
            let classification = get_market_classification(raw.open, raw.high, raw.low, raw.close);
            let window_start = raw.trading_date - Duration::days(7);
            
            // Clean up history and calculate Prior Week High/Low
            high_history.retain(|(date, _)| date >= &window_start);
            low_history.retain(|(date, _)| date >= &window_start);
            
            let prior_week_high = high_history.iter().map(|(_, h)| *h).max_by(|a, b| a.partial_cmp(b).unwrap());
            let prior_week_low = low_history.iter().map(|(_, l)| *l).min_by(|a, b| a.partial_cmp(b).unwrap());

            daily_views.push(dm::ClassifiedDailyView {
                trading_date: raw.trading_date,
                asset_id: raw.asset_id.clone(),
                dow: raw.trading_date.format("%a").to_string(),
                day_type: classification,
                open: raw.open, high: raw.high, low: raw.low, close: raw.close,
                volume: raw.volume, bars: raw.bars,
                start_ts: raw.start_ts, end_ts: raw.end_ts,
                high_ts: raw.high_ts, high_session: raw.high_session,
                low_ts: raw.low_ts, low_session: raw.low_session,
                high_bar: raw.high_bar, low_bar: raw.low_bar,
                prior_day_high,
                prior_day_low,
                prior_week_high,
                prior_week_low,
            });

            // Update lookback state for next iteration
            prior_day_high = Some(raw.high);
            prior_day_low = Some(raw.low);
            high_history.push((raw.trading_date, raw.high));
            low_history.push((raw.trading_date, raw.low));
        }

        let limit = hwm.unwrap_or(NaiveDate::MIN);
        Ok(daily_views.into_iter().filter(|v| v.trading_date >= limit).collect())
    }

    pub async fn weekly_views(&self, asset_id: &str) -> Result<Vec<dm::ClassifiedWeeklyView>> {
        let hwm: Option<NaiveDate> = self.get_hwm("weekly_views", "week_start", asset_id).await?;
        
        let daily_data: Vec<dm::RawDailyView> = self.fetch_buffered(
            "SELECT * FROM daily_views WHERE asset_id = $1", 
            asset_id, hwm, Duration::days(7),
            "trading_date" // Explicitly specify the filter column

        ).await?;

        let mut weekly_map: HashMap<NaiveDate, dm::ClassifiedWeeklyView> = HashMap::new();

        for d in daily_data {
            let week_start = d.trading_date - Duration::days(d.trading_date.weekday().num_days_from_monday() as i64);
            
            let entry = weekly_map.entry(week_start).or_insert_with(|| dm::ClassifiedWeeklyView {
                week_start, asset_id: d.asset_id.clone(), month_of_year: week_start.month() as i32,
                open: d.open, high: d.high, low: d.low, close: d.close, volume: 0, bars: 0,
                high_trading_date: d.trading_date, high_ts: d.high_ts, high_session: d.high_session.clone(),
                low_trading_date: d.trading_date, low_ts: d.low_ts, low_session: d.low_session.clone(),
                weekly_type: MarketType::Other,
            });

            entry.close = d.close;
            entry.volume += d.volume;
            entry.bars += d.bars;

            if d.high > entry.high {
                entry.high = d.high;
                entry.high_trading_date = d.trading_date;
                entry.high_ts = d.high_ts;
                entry.high_session = d.high_session.clone();
            }
            if d.low < entry.low {
                entry.low = d.low;
                entry.low_trading_date = d.trading_date;
                entry.low_ts = d.low_ts;
                entry.low_session = d.low_session.clone();
            }
        }

        let limit = hwm.unwrap_or(NaiveDate::MIN);
        let mut result: Vec<_> = weekly_map.into_values().map(|mut w| {
            w.weekly_type = get_market_classification(w.open, w.high, w.low, w.close);
            w
        }).filter(|w| w.week_start >= limit).collect();
        
        result.sort_by_key(|v| v.week_start);
        Ok(result)
    }

    pub async fn monthly_views(
        &self,
        asset_id: &str,
    ) -> Result<Vec<dm::ClassifiedMonthlyView>> {
        // 1. Get HWM from the target table
        let hwm: Option<NaiveDate> = self.get_hwm("monthly_views", "month_start", asset_id).await?;
        
        // 2. Fetch Daily Views with a buffer to re-aggregate the current month
        let daily_data: Vec<dm::RawDailyView> = self.fetch_buffered(
            "SELECT * FROM daily_views WHERE asset_id = $1", 
            asset_id, 
            hwm, 
            Duration::days(31), // Buffer for a full month
            "trading_date" // Explicitly specify the filter column
        ).await?;

        if daily_data.is_empty() { return Ok(Vec::new()); }

        let mut monthly_map: HashMap<NaiveDate, dm::ClassifiedMonthlyView> = HashMap::new();

        for d in daily_data {
            // Normalize trading_date to the 1st of the month for grouping
            let month_start = d.trading_date.with_day(1).unwrap();
            
            let entry = monthly_map.entry(month_start).or_insert_with(|| dm::ClassifiedMonthlyView {
                month_start,
                asset_id: d.asset_id.clone(),
                open: d.open,
                high: d.high,
                low: d.low,
                close: d.close,
                volume: 0,
                bars: 0,
                monthly_type: MarketType::Other,
                start_trading_date: d.trading_date,
                end_trading_date: d.trading_date,
                high_trading_date: d.trading_date,
                high_ts: d.high_ts,
                high_session: d.high_session.clone(),
                low_trading_date: d.trading_date,
                low_ts: d.low_ts,
                low_session: d.low_session.clone(),
            });

            // Update aggregation state
            entry.close = d.close;
            entry.end_trading_date = d.trading_date;
            entry.volume += d.volume;
            entry.bars += d.bars;

            if d.high > entry.high {
                entry.high = d.high;
                entry.high_trading_date = d.trading_date;
                entry.high_ts = d.high_ts;
                entry.high_session = d.high_session.clone();
            }
            if d.low < entry.low {
                entry.low = d.low;
                entry.low_trading_date = d.trading_date;
                entry.low_ts = d.low_ts;
                entry.low_session = d.low_session.clone();
            }
        }

        let limit = hwm.unwrap_or(NaiveDate::MIN);
        let mut result: Vec<_> = monthly_map.into_values().map(|mut m| {
            m.monthly_type = get_market_classification(m.open, m.high, m.low, m.close);
            m
        }).filter(|m| m.month_start >= limit).collect();

        result.sort_by_key(|v| v.month_start);
        Ok(result)
    }
    

    pub async fn yearly_views(
        &self,
        asset_id: &str,
    ) -> Result<Vec<dm::ClassifiedYearlyView>> {
        // 1. Get HWM from the target table
        let hwm: Option<NaiveDate> = self.get_hwm("yearly_views", "year_start", asset_id).await?;
        
        // 2. Fetch Monthly Views (no buffer needed usually, or 365 days for safety)
        let monthly_data: Vec<dm::RawMonthlyView> = self.fetch_buffered(
            "SELECT * FROM monthly_views WHERE asset_id = $1", 
            asset_id, 
            hwm, 
            Duration::days(365),
            "month_start" // Explicitly specify the filter column
        ).await?;

        if monthly_data.is_empty() { return Ok(Vec::new()); }

        let mut yearly_map: HashMap<i32, dm::ClassifiedYearlyView> = HashMap::new();

        for m in monthly_data {
            let year = m.month_start.year();
            let year_start = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();

            let entry = yearly_map.entry(year).or_insert_with(|| dm::ClassifiedYearlyView {
                year_start,
                asset_id: m.asset_id.clone(),
                open: m.open,
                high: m.high,
                low: m.low,
                close: m.close,
                volume: 0,
                bars: 0,
                yearly_type: MarketType::Other,
                high_trading_date: m.high_trading_date,
                high_ts: m.high_ts,
                high_session: m.high_session.clone(),
                low_trading_date: m.low_trading_date,
                low_ts: m.low_ts,
                low_session: m.low_session.clone(),
            });

            // Update aggregation state
            entry.close = m.close;
            entry.volume += m.volume;
            entry.bars += m.bars;

            if m.high > entry.high {
                entry.high = m.high;
                entry.high_trading_date = m.high_trading_date;
                entry.high_ts = m.high_ts;
                entry.high_session = m.high_session.clone();
            }
            if m.low < entry.low {
                entry.low = m.low;
                entry.low_trading_date = m.low_trading_date;
                entry.low_ts = m.low_ts;
                entry.low_session = m.low_session.clone();
            }
        }

        let limit = hwm.unwrap_or(NaiveDate::MIN);
        let mut result: Vec<_> = yearly_map.into_values().map(|mut y| {
            y.yearly_type = get_market_classification(y.open, y.high, y.low, y.close);
            y
        }).filter(|y| y.year_start >= limit).collect();

        result.sort_by_key(|v| v.year_start);
        Ok(result)
    }

}



