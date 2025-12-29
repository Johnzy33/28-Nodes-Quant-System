
use sqlx::{FromRow};
use shared_models::models as sm;



pub trait FetchableML: for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin {
    fn query() -> &'static str;
}



impl FetchableML for sm::SessionContextData {
    fn query() -> &'static str {
        r#"
        SELECT *
        FROM session_context
        WHERE asset_id = $1 AND cs_bias IS NOT NULL AND cs_bias != 'Other'
        ORDER BY trading_date ASC, session_end_ts ASC
        "#
    }
}

impl FetchableML for sm::EightContextData {
    fn query() -> &'static str {
        r#"
        SELECT *
        FROM block_context
        WHERE asset_id = $1 AND cb_bias IS NOT NULL AND cb_bias != 'Other'
        ORDER BY trading_date ASC, session_end_ts ASC
        "#
    }
}

impl FetchableML for sm::DailyContextData {
    fn query() -> &'static str {
        r#"
        SELECT trading_date, asset_id, day_type, high_session, low_session, high_bar, low_bar      
        FROM daily_views
        WHERE asset_id = $1 AND day_type IS NOT NULL AND day_type != 'Other'
        ORDER BY trading_date ASC
        "#
    }
}


