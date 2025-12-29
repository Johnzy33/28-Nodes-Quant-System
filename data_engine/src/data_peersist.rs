use anyhow::{anyhow, Result};
use sqlx::{QueryBuilder, Postgres};
use crate::data_fetch::{
    ClassifiedSession, ClassifiedDailyView, ClassifiedWeeklyView, 
    ClassifiedMonthlyView, ClassifiedYearlyView, Classified8HrBlock, 
};
use shared_models::models::{SessionContextData, EightContextData};
use log::info;

impl super::DataService {

    // ====================================================================
    // L1: Classified Sessions Persistence
    // ====================================================================
    
    /// Bulk upserts ClassifiedSession data into the session_views table.
    pub async fn persist_classified_sessions(&self, sessions: &[ClassifiedSession]) -> Result<()> {
        if sessions.is_empty() { return Ok(()); }
        
        // Use a safe chunk size
        for chunk in sessions.chunks(3000) {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO session_views (
                    trading_date, asset_id, session_name, session_type,
                    start_ts, end_ts, open, high, high_ts, low, low_ts, close, volume, bars
                ) "
            );

            // 1. Push Values
            query_builder.push_values(chunk, |mut b, s| {
                b.push_bind(s.trading_date)
                .push_bind(&s.asset_id)
                .push_bind(&s.session_name)
                .push_bind(s.session_type.to_string()) 
                .push_bind(s.start_ts)
                .push_bind(s.end_ts)
                .push_bind(s.open)
                .push_bind(s.high)
                .push_bind(s.high_ts)
                .push_bind(s.low)
                .push_bind(s.low_ts)
                .push_bind(s.close)
                .push_bind(s.volume)
                .push_bind(s.bars);
            });

            // 2. Push the On Conflict Clause (Manual SET assignment)
            query_builder.push(
                " ON CONFLICT (trading_date, asset_id, session_name) DO UPDATE SET
                    session_type = EXCLUDED.session_type,
                    start_ts = EXCLUDED.start_ts,
                    end_ts = EXCLUDED.end_ts,
                    open = EXCLUDED.open,
                    high = EXCLUDED.high,
                    high_ts = EXCLUDED.high_ts,
                    low = EXCLUDED.low,
                    low_ts = EXCLUDED.low_ts,
                    close = EXCLUDED.close,
                    volume = EXCLUDED.volume,
                    bars = EXCLUDED.bars"
            );

            // 3. Execute the built query
            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow!("Failed to batch upsert sessions: {}", e))?;
        }
        info!("Successfully persisted {} Classified Sessions (L1).", sessions.len());
        Ok(())
    }

    pub async fn persist_session_context(&self, contexts: &[SessionContextData]) -> Result<()> {
    if contexts.is_empty() { 
        return Ok(()); 
    }
    
    // Use a safe chunk size
    for chunk in contexts.chunks(3000) {
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO session_context (
                trading_date, asset_id, session_end_ts, 
                cs_name, cs_bias, 
                ps1_name, ps1_bias, 
                ps2_name, ps2_bias
            ) "
        );

        // 1. Push Values (Bind parameters from SessionContextData struct)
        query_builder.push_values(chunk, |mut b, s| {
            b.push_bind(s.trading_date)
            .push_bind(&s.asset_id)
            .push_bind(s.session_end_ts)
            // Current Session (CS)
            .push_bind(&s.cs_name)
            .push_bind(&s.cs_bias) 
            // Preceding Session 1 (PS1)
            .push_bind(&s.ps1_name)
            .push_bind(&s.ps1_bias)
            // Preceding Session 2 (PS2)
            .push_bind(&s.ps2_name)
            .push_bind(&s.ps2_bias);
        });

        // 2. Push the On Conflict Clause
        // The unique constraint is on (trading_date, asset_id, cs_name).
        query_builder.push(
            " ON CONFLICT (trading_date, asset_id, cs_name) DO UPDATE SET
                cs_bias = EXCLUDED.cs_bias,
                ps1_name = EXCLUDED.ps1_name,
                ps1_bias = EXCLUDED.ps1_bias,
                ps2_name = EXCLUDED.ps2_name,
                ps2_bias = EXCLUDED.ps2_bias,
                session_end_ts = EXCLUDED.session_end_ts" // Update timestamp if necessary
        );

        // 3. Execute the built query
        let query = query_builder.build();
        query.execute(&self.pool).await
            .map_err(|e| anyhow!("Failed to batch upsert session context: {}", e))?;
    }
    
    info!("Successfully persisted {} Session Context records (L1.5).", contexts.len());
    Ok(())
    }

    pub async fn persist_8hr_blocks(&self, blocks: &[Classified8HrBlock]) -> Result<()> {
        if blocks.is_empty() {
            return Ok(());
        }

        // Use a safe chunk size
        for chunk in blocks.chunks(3000) {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO block_base (
                    trading_date, asset_id, block_number, start_ts, end_ts,
                    open, high, low, close, volume, bars,
                    block_type
                ) "
            );

            // 1. Push Values
            query_builder.push_values(chunk, |mut b, s| {
                b.push_bind(s.trading_date)
                .push_bind(&s.asset_id)
                .push_bind(s.block_number)
                .push_bind(s.start_ts)
                .push_bind(s.end_ts)
                .push_bind(s.open)
                .push_bind(s.high)
               // .push_bind(s.high_ts)
                .push_bind(s.low)
           //     .push_bind(s.low_ts)
                .push_bind(s.close)
                .push_bind(s.volume)
                .push_bind(s.bars)
                .push_bind(s.block_type.to_string()); 
                
            });

            // 2. Push the On Conflict Clause (Manual SET assignment)
            query_builder.push(
                " ON CONFLICT (trading_date, asset_id, block_number) DO UPDATE SET
                    start_ts = EXCLUDED.start_ts,
                    end_ts = EXCLUDED.end_ts,
                    open = EXCLUDED.open,
                    high = EXCLUDED.high,
                    low = EXCLUDED.low,
                    close = EXCLUDED.close,
                    volume = EXCLUDED.volume,
                    bars = EXCLUDED.bars,
                    block_type = EXCLUDED.block_type
                "
                    
            );

            // 3. Execute the built query
            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow!("Failed to batch upsert 8hr blocks: {}", e))?;
        }
        info!("Successfully persisted {} Classified 8-Hour Blocks.", blocks.len());
        Ok(())
    }


    pub async fn persist_block_context(&self, contexts: &[EightContextData]) -> Result<()> {
        if contexts.is_empty() { 
            return Ok(()); 
        }
        
        // Use a safe chunk size (3000 is typically good for Postgres batch inserts)
        for chunk in contexts.chunks(3000) {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO block_context (
                    trading_date, asset_id, session_end_ts, 
                    cb_num, cb_bias, 
                    pb1_num, pb1_bias, 
                    pb2_num, pb2_bias
                ) "
            );

            // 1. Push Values (Bind parameters from EightContextData struct)
            query_builder.push_values(chunk, |mut b, s| {
                b.push_bind(s.trading_date)
                .push_bind(&s.asset_id)
                .push_bind(s.session_end_ts)
                // Current Block (CB)
                .push_bind(&s.cb_num)
                .push_bind(&s.cb_bias) 
                // Preceding Block 1 (PB1)
                .push_bind(&s.pb1_num)
                .push_bind(&s.pb1_bias)
                // Preceding Block 2 (PB2)
                .push_bind(&s.pb2_num)
                .push_bind(&s.pb2_bias);
            });

            // 2. Push the On Conflict Clause
            // The unique constraint is assumed to be on (trading_date, asset_id, cb_num).
            query_builder.push(
                " ON CONFLICT (trading_date, asset_id, cb_num) DO UPDATE SET
                    cb_bias = EXCLUDED.cb_bias,
                    pb1_num = EXCLUDED.pb1_num,
                    pb1_bias = EXCLUDED.pb1_bias,
                    pb2_num = EXCLUDED.pb2_num,
                    pb2_bias = EXCLUDED.pb2_bias,
                    session_end_ts = EXCLUDED.session_end_ts" // Update timestamp
            );

            // 3. Execute the built query
            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow!("Failed to batch upsert block context: {}", e))?;
        }
        
        info!("Successfully persisted {} Block Context records.", contexts.len());
        Ok(())
    }

    // ====================================================================
    // L2: Classified Daily Views Persistence
    // ====================================================================

    /// Bulk upserts ClassifiedDailyView data into the daily_views table.
    pub async fn persist_classified_daily_views(&self, views: &[ClassifiedDailyView]) -> Result<()> {
        if views.is_empty() { return Ok(()) }

        for chunk in views.chunks(3000) {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO daily_views (
                    trading_date, asset_id, dow, day_type, 
                    open, high, low, close, volume, bars, start_ts, end_ts, 
                    high_ts, high_session, low_ts, low_session, high_bar, low_bar,
                    prior_day_high, prior_day_low, prior_week_high, prior_week_low
                ) "
            );

            // 1. Push Values
            query_builder.push_values(chunk.iter(), |mut b, view| {
                b.push_bind(view.trading_date)
                 .push_bind(&view.asset_id)
                 .push_bind(&view.dow)
                 .push_bind(view.day_type.to_string())
                 .push_bind(view.open)
                 .push_bind(view.high)
                 .push_bind(view.low)
                 .push_bind(view.close)
                 .push_bind(view.volume)
                 .push_bind(view.bars)
                 .push_bind(view.start_ts)
                 .push_bind(view.end_ts)
                 .push_bind(view.high_ts)
                 .push_bind(&view.high_session)
                 .push_bind(view.low_ts)
                 .push_bind(&view.low_session)
                 .push_bind(view.high_bar)
                 .push_bind(view.low_bar)
                 .push_bind(view.prior_day_high)
                 .push_bind(view.prior_day_low)
                 .push_bind(view.prior_week_high)
                 .push_bind(view.prior_week_low);
            });

            // 2. Define the ON CONFLICT (UPSERT) clause (Manual SET assignment)
            query_builder.push(
                " ON CONFLICT (trading_date, asset_id) DO UPDATE SET 
                    dow = EXCLUDED.dow, 
                    day_type = EXCLUDED.day_type, 
                    open = EXCLUDED.open, 
                    high = EXCLUDED.high, 
                    low = EXCLUDED.low, 
                    close = EXCLUDED.close,
                    volume = EXCLUDED.volume, 
                    bars = EXCLUDED.bars, 
                    start_ts = EXCLUDED.start_ts, 
                    end_ts = EXCLUDED.end_ts,
                    high_ts = EXCLUDED.high_ts, 
                    high_session = EXCLUDED.high_session, 
                    low_ts = EXCLUDED.low_ts, 
                    low_session = EXCLUDED.low_session,
                    high_bar = EXCLUDED.high_bar,
                    low_bar = EXCLUDED.low_bar,
                    prior_day_high = EXCLUDED.prior_day_high, 
                    prior_day_low = EXCLUDED.prior_day_low, 
                    prior_week_high = EXCLUDED.prior_week_high, 
                    prior_week_low = EXCLUDED.prior_week_low"
            );

            // 3. Execute
            query_builder.build()
                .execute(&self.pool).await
                .map_err(|e| anyhow!("Failed to batch upsert daily views: {}", e))?;
        }
        info!("Successfully persisted {} Classified Daily Views (L2).", views.len());
        Ok(())
    }

    // ====================================================================
    // L3: Classified Weekly Views Persistence
    // ====================================================================

    /// Bulk upserts ClassifiedWeeklyView data into the weekly_views table.
    pub async fn persist_classified_weekly_views(&self, views: &[ClassifiedWeeklyView]) -> Result<()> {
        if views.is_empty() { return Ok(()) }

        for chunk in views.chunks(3000) {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO weekly_views (
                    week_start, asset_id, month_of_year, weekly_type,
                    open, high, low, close, volume, bars, 
                    high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session
                ) "
            );

            // 1. Push Values
            query_builder.push_values(chunk.iter(), |mut b, view| {
                b.push_bind(view.week_start)
                .push_bind(&view.asset_id)
                .push_bind(view.month_of_year)
                .push_bind(view.weekly_type.to_string())
                .push_bind(view.open)
                .push_bind(view.high)
                .push_bind(view.low)
                .push_bind(view.close)
                .push_bind(view.volume)
                .push_bind(view.bars)
                .push_bind(view.high_trading_date)
                .push_bind(view.high_ts)
                .push_bind(&view.high_session)
                .push_bind(view.low_trading_date)
                .push_bind(view.low_ts)
                .push_bind(&view.low_session);
            });

            // 2. Define the ON CONFLICT (UPSERT) clause (Manual SET assignment)
            query_builder.push(
                " ON CONFLICT (week_start, asset_id) DO UPDATE SET 
                    month_of_year = EXCLUDED.month_of_year, 
                    weekly_type = EXCLUDED.weekly_type, 
                    open = EXCLUDED.open, 
                    high = EXCLUDED.high, 
                    low = EXCLUDED.low, 
                    close = EXCLUDED.close,
                    volume = EXCLUDED.volume, 
                    bars = EXCLUDED.bars,
                    high_trading_date = EXCLUDED.high_trading_date, 
                    high_ts = EXCLUDED.high_ts, 
                    high_session = EXCLUDED.high_session,
                    low_trading_date = EXCLUDED.low_trading_date, 
                    low_ts = EXCLUDED.low_ts, 
                    low_session = EXCLUDED.low_session"
            );

            // 3. Execute
            query_builder.build()
                .execute(&self.pool).await
                .map_err(|e| anyhow!("Failed to batch upsert weekly views: {}", e))?;
        }
        info!("Successfully persisted {} Classified Weekly Views (L3).", views.len());
        Ok(())
    }
    
    // ====================================================================
    // L4: Classified Monthly Views Persistence
    // ====================================================================

    /// Bulk upserts ClassifiedMonthlyView data into the monthly_views table.
    pub async fn persist_classified_monthly_views(&self, views: &[ClassifiedMonthlyView]) -> Result<()> {
        if views.is_empty() { return Ok(()) }

        for chunk in views.chunks(3000) {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO monthly_views (
                    month_start, asset_id, monthly_type,
                    open, high, low, close, volume, bars, 
                    start_trading_date, end_trading_date,
                    high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session
                ) "
            );

            // 1. Push Values
            query_builder.push_values(chunk.iter(), |mut b, view| {
                b.push_bind(view.month_start)
                .push_bind(&view.asset_id)
                .push_bind(view.monthly_type.to_string())
                .push_bind(view.open)
                .push_bind(view.high)
                .push_bind(view.low)
                .push_bind(view.close)
                .push_bind(view.volume)
                .push_bind(view.bars)
                .push_bind(view.start_trading_date)
                .push_bind(view.end_trading_date)
                .push_bind(view.high_trading_date)
                .push_bind(view.high_ts)
                .push_bind(&view.high_session)
                .push_bind(view.low_trading_date)
                .push_bind(view.low_ts)
                .push_bind(&view.low_session);
            });

            // 2. Define the ON CONFLICT (UPSERT) clause (Manual SET assignment)
            query_builder.push(
                " ON CONFLICT (month_start, asset_id) DO UPDATE SET 
                    monthly_type = EXCLUDED.monthly_type, 
                    open = EXCLUDED.open, 
                    high = EXCLUDED.high, 
                    low = EXCLUDED.low, 
                    close = EXCLUDED.close,
                    volume = EXCLUDED.volume, 
                    bars = EXCLUDED.bars,
                    start_trading_date = EXCLUDED.start_trading_date, 
                    end_trading_date = EXCLUDED.end_trading_date,
                    high_trading_date = EXCLUDED.high_trading_date, 
                    high_ts = EXCLUDED.high_ts, 
                    high_session = EXCLUDED.high_session,
                    low_trading_date = EXCLUDED.low_trading_date, 
                    low_ts = EXCLUDED.low_ts, 
                    low_session = EXCLUDED.low_session"
            );

            // 3. Execute
            query_builder.build()
                .execute(&self.pool).await
                .map_err(|e| anyhow!("Failed to batch upsert monthly views: {}", e))?;
        }
        info!("Successfully persisted {} Classified Monthly Views (L4).", views.len());
        Ok(())
    }

    // ====================================================================
    // L5: Classified Yearly Views Persistence
    // ====================================================================

    /// Bulk upserts ClassifiedYearlyView data into the yearly_views table.
    pub async fn persist_classified_yearly_views(&self, views: &[ClassifiedYearlyView]) -> Result<()> {
        if views.is_empty() { return Ok(()) }

        for chunk in views.chunks(4000) {
            let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO yearly_views (
                    year_start, asset_id, yearly_type,
                    open, high, low, close, volume, bars, 
                    high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session
                ) "
            );
            
            // 1. Push Values
            query_builder.push_values(chunk.iter(), |mut b, view| {
                b.push_bind(view.year_start)
                .push_bind(&view.asset_id)
                .push_bind(view.yearly_type.to_string())
                .push_bind(view.open)
                .push_bind(view.high)
                .push_bind(view.low)
                .push_bind(view.close)
                .push_bind(view.volume)
                .push_bind(view.bars)
                .push_bind(view.high_trading_date)
                .push_bind(view.high_ts)
                .push_bind(&view.high_session)
                .push_bind(view.low_trading_date)
                .push_bind(view.low_ts)
                .push_bind(&view.low_session);
            });

            // 2. Define the ON CONFLICT (UPSERT) clause (Manual SET assignment)
            query_builder.push(
                " ON CONFLICT (year_start, asset_id) DO UPDATE SET 
                    yearly_type = EXCLUDED.yearly_type, 
                    open = EXCLUDED.open, 
                    high = EXCLUDED.high, 
                    low = EXCLUDED.low, 
                    close = EXCLUDED.close,
                    volume = EXCLUDED.volume, 
                    bars = EXCLUDED.bars,
                    high_trading_date = EXCLUDED.high_trading_date, 
                    high_ts = EXCLUDED.high_ts, 
                    high_session = EXCLUDED.high_session,
                    low_trading_date = EXCLUDED.low_trading_date, 
                    low_ts = EXCLUDED.low_ts, 
                    low_session = EXCLUDED.low_session"
            );

            // 3. Execute
            query_builder.build()
                .execute(&self.pool).await
                .map_err(|e| anyhow!("Failed to batch upsert yearly views: {}", e))?;
        }
        info!("Successfully persisted {} Classified Yearly Views (L5).", views.len());
        Ok(())
    }


    

}