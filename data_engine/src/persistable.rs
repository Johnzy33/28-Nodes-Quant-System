
use sqlx::{Postgres, query_builder::Separated};
use shared_models::models as sm;
use crate::data_model as dm;

pub trait Persistable {
    fn table_name() -> &'static str;
    fn column_names() -> &'static str;
    fn conflict_keys() -> &'static str;
    fn update_columns() -> &'static str;
    
    // We bind to the 'args lifetime of the Postgres backend
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>);
}
// ====================================================================
// Context Data
// ====================================================================

impl Persistable for sm::SessionContextData {
    fn table_name() -> &'static str { "session_context" }
    fn column_names() -> &'static str { 
        "trading_date, asset_id, session_end_ts, cs_name, cs_bias, ps1_name, ps1_bias, ps2_name, ps2_bias" 
    }
    fn conflict_keys() -> &'static str { "trading_date, asset_id, cs_name" }
    fn update_columns() -> &'static str {
        "cs_bias = EXCLUDED.cs_bias, ps1_name = EXCLUDED.ps1_name, ps1_bias = EXCLUDED.ps1_bias, ps2_name = EXCLUDED.ps2_name, ps2_bias = EXCLUDED.ps2_bias, session_end_ts = EXCLUDED.session_end_ts"
    }
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>) {
        b.push_bind(self.trading_date).push_bind(&self.asset_id).push_bind(self.session_end_ts)
         .push_bind(&self.cs_name).push_bind(&self.cs_bias)
         .push_bind(&self.ps1_name).push_bind(&self.ps1_bias)
         .push_bind(&self.ps2_name).push_bind(&self.ps2_bias);
    }
}

impl Persistable for sm::EightContextData {
    fn table_name() -> &'static str { "block_context" }
    fn column_names() -> &'static str { 
        "trading_date, asset_id, session_end_ts, cb_num, cb_bias, pb1_num, pb1_bias, pb2_num, pb2_bias" 
    }
    fn conflict_keys() -> &'static str { "trading_date, asset_id, cb_num" }
    fn update_columns() -> &'static str {
        "cb_bias = EXCLUDED.cb_bias, pb1_num = EXCLUDED.pb1_num, pb1_bias = EXCLUDED.pb1_bias, pb2_num = EXCLUDED.pb2_num, pb2_bias = EXCLUDED.pb2_bias, session_end_ts = EXCLUDED.session_end_ts"
    }
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>) {
        b.push_bind(self.trading_date).push_bind(&self.asset_id).push_bind(self.session_end_ts)
         .push_bind(self.cb_num).push_bind(&self.cb_bias)
         .push_bind(self.pb1_num).push_bind(&self.pb1_bias)
         .push_bind(self.pb2_num).push_bind(&self.pb2_bias);
    }
}

// ====================================================================
// Classified Views Data
// ====================================================================

impl Persistable for dm::ClassifiedSession {
    fn table_name() -> &'static str { "session_views" }
    fn column_names() -> &'static str { 
        "trading_date, asset_id, session_name, session_type, start_ts, end_ts, open, high, high_ts, low, low_ts, close, volume, bars" 
    }
    fn conflict_keys() -> &'static str { "trading_date, asset_id, session_name" }
    fn update_columns() -> &'static str {
        "session_type = EXCLUDED.session_type, start_ts = EXCLUDED.start_ts, end_ts = EXCLUDED.end_ts, open = EXCLUDED.open, high = EXCLUDED.high, high_ts = EXCLUDED.high_ts, low = EXCLUDED.low, low_ts = EXCLUDED.low_ts, close = EXCLUDED.close, volume = EXCLUDED.volume, bars = EXCLUDED.bars"
    }
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>) {
        b.push_bind(self.trading_date).push_bind(&self.asset_id).push_bind(&self.session_name)
         .push_bind(self.session_type.to_string()).push_bind(self.start_ts).push_bind(self.end_ts)
         .push_bind(self.open).push_bind(self.high).push_bind(self.high_ts)
         .push_bind(self.low).push_bind(self.low_ts).push_bind(self.close)
         .push_bind(self.volume).push_bind(self.bars);
    }
}

impl Persistable for dm::Classified8HrBlock {
    fn table_name() -> &'static str { "block_base" }
    fn column_names() -> &'static str { 
        "trading_date, asset_id, block_number, start_ts, end_ts, open, high, low, close, volume, bars, block_type" 
    }
    fn conflict_keys() -> &'static str { "trading_date, asset_id, block_number" }
    fn update_columns() -> &'static str {
        "start_ts = EXCLUDED.start_ts, end_ts = EXCLUDED.end_ts, open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low, close = EXCLUDED.close, volume = EXCLUDED.volume, bars = EXCLUDED.bars, block_type = EXCLUDED.block_type"
    }
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>) {
        b.push_bind(self.trading_date).push_bind(&self.asset_id).push_bind(self.block_number)
         .push_bind(self.start_ts).push_bind(self.end_ts).push_bind(self.open)
         .push_bind(self.high).push_bind(self.low).push_bind(self.close)
         .push_bind(self.volume).push_bind(self.bars).push_bind(self.block_type.to_string());
    }
}

impl Persistable for dm::ClassifiedDailyView {
    fn table_name() -> &'static str { "daily_views" }
    fn column_names() -> &'static str { 
        "trading_date, asset_id, dow, day_type, open, high, low, close, volume, bars, start_ts, end_ts, high_ts, high_session, low_ts, low_session, high_bar, low_bar, prior_day_high, prior_day_low, prior_week_high, prior_week_low" 
    }
    fn conflict_keys() -> &'static str { "trading_date, asset_id" }
    fn update_columns() -> &'static str {
        "dow = EXCLUDED.dow, day_type = EXCLUDED.day_type, open = EXCLUDED.open, high = EXCLUDED.high, low = EXCLUDED.low, close = EXCLUDED.close, volume = EXCLUDED.volume, bars = EXCLUDED.bars, high_bar = EXCLUDED.high_bar, low_bar = EXCLUDED.low_bar, prior_day_high = EXCLUDED.prior_day_high, prior_day_low = EXCLUDED.prior_day_low, prior_week_high = EXCLUDED.prior_week_high, prior_week_low = EXCLUDED.prior_week_low"
    }
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>) {
        b.push_bind(self.trading_date).push_bind(&self.asset_id).push_bind(&self.dow).push_bind(self.day_type.to_string())
         .push_bind(self.open).push_bind(self.high).push_bind(self.low).push_bind(self.close)
         .push_bind(self.volume).push_bind(self.bars).push_bind(self.start_ts).push_bind(self.end_ts)
         .push_bind(self.high_ts).push_bind(&self.high_session).push_bind(self.low_ts).push_bind(&self.low_session)
         .push_bind(self.high_bar).push_bind(self.low_bar)
         .push_bind(self.prior_day_high).push_bind(self.prior_day_low)
         .push_bind(self.prior_week_high).push_bind(self.prior_week_low);
    }
}

impl Persistable for dm::ClassifiedWeeklyView {
    fn table_name() -> &'static str { "weekly_views" }
    fn column_names() -> &'static str { 
        "week_start, asset_id, month_of_year, weekly_type, open, high, low, close, volume, bars, high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session" 
    }
    fn conflict_keys() -> &'static str { "week_start, asset_id" }
    fn update_columns() -> &'static str {
        "month_of_year = EXCLUDED.month_of_year, weekly_type = EXCLUDED.weekly_type, close = EXCLUDED.close, high = EXCLUDED.high, low = EXCLUDED.low, volume = EXCLUDED.volume, bars = EXCLUDED.bars, high_ts = EXCLUDED.high_ts, low_ts = EXCLUDED.low_ts"
    }
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>) {
        b.push_bind(self.week_start).push_bind(&self.asset_id).push_bind(self.month_of_year).push_bind(self.weekly_type.to_string())
         .push_bind(self.open).push_bind(self.high).push_bind(self.low).push_bind(self.close).push_bind(self.volume).push_bind(self.bars)
         .push_bind(self.high_trading_date).push_bind(self.high_ts).push_bind(&self.high_session)
         .push_bind(self.low_trading_date).push_bind(self.low_ts).push_bind(&self.low_session);
    }
}

impl Persistable for dm::ClassifiedMonthlyView {
    fn table_name() -> &'static str { "monthly_views" }
    fn column_names() -> &'static str { 
        "month_start, asset_id, monthly_type, open, high, low, close, volume, bars, start_trading_date, end_trading_date, high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session" 
    }
    fn conflict_keys() -> &'static str { "month_start, asset_id" }
    fn update_columns() -> &'static str {
        "monthly_type = EXCLUDED.monthly_type, close = EXCLUDED.close, high = EXCLUDED.high, low = EXCLUDED.low, volume = EXCLUDED.volume, bars = EXCLUDED.bars, end_trading_date = EXCLUDED.end_trading_date"
    }
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>) {
        b.push_bind(self.month_start).push_bind(&self.asset_id).push_bind(self.monthly_type.to_string())
         .push_bind(self.open).push_bind(self.high).push_bind(self.low).push_bind(self.close).push_bind(self.volume).push_bind(self.bars)
         .push_bind(self.start_trading_date).push_bind(self.end_trading_date)
         .push_bind(self.high_trading_date).push_bind(self.high_ts).push_bind(&self.high_session)
         .push_bind(self.low_trading_date).push_bind(self.low_ts).push_bind(&self.low_session);
    }
}

impl Persistable for dm::ClassifiedYearlyView {
    fn table_name() -> &'static str { "yearly_views" }
    fn column_names() -> &'static str { 
        "year_start, asset_id, yearly_type, open, high, low, close, volume, bars, high_trading_date, high_ts, high_session, low_trading_date, low_ts, low_session" 
    }
    fn conflict_keys() -> &'static str { "year_start, asset_id" }
    fn update_columns() -> &'static str {
        "yearly_type = EXCLUDED.yearly_type, close = EXCLUDED.close, high = EXCLUDED.high, low = EXCLUDED.low, volume = EXCLUDED.volume, bars = EXCLUDED.bars"
    }
    fn bind_values<'args>(&'args self, b: &mut Separated<'_, 'args, Postgres, &str>) {
        b.push_bind(self.year_start).push_bind(&self.asset_id).push_bind(self.yearly_type.to_string())
         .push_bind(self.open).push_bind(self.high).push_bind(self.low).push_bind(self.close).push_bind(self.volume).push_bind(self.bars)
         .push_bind(self.high_trading_date).push_bind(self.high_ts).push_bind(&self.high_session)
         .push_bind(self.low_trading_date).push_bind(self.low_ts).push_bind(&self.low_session);
    }
}


