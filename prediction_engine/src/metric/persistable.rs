
use sqlx::{Postgres, query_builder::Separated};
use crate::metric::metrics_model as mm;

pub trait PersistableML {
    fn insert_sql() -> &'static str;
    fn conflict_sql() -> &'static str;
    fn bind_values<'a>(&'a self, b: &mut Separated<'_, 'a, Postgres, &str>);
}

macro_rules! impl_persistable {
    ($t:ty, $insert:expr, $conflict:expr, $($field:ident),*) => {
        impl PersistableML for $t {
            fn insert_sql() -> &'static str { $insert }
            fn conflict_sql() -> &'static str { $conflict }
            fn bind_values<'a>(&'a self, b: &mut Separated<'_, 'a, Postgres, &str>) {
                $( b.push_bind(&self.$field); )*
                b.push_bind(self.p_6m).push_bind(self.count_6m).push_bind(self.total_6m)
                 .push_bind(self.p_1y).push_bind(self.count_1y).push_bind(self.total_1y)
                 .push_bind(self.p_all).push_bind(self.count_all).push_bind(self.total_all);
            }
        }
    };
}

macro_rules! impl_score_persistable {
    ($t:ty, $insert:expr, $conflict:expr, $s6:ident, $s1:ident, $sa:ident, $($field:ident),*) => {
        impl PersistableML for $t {
            fn insert_sql() -> &'static str { $insert }
            fn conflict_sql() -> &'static str { $conflict }
            fn bind_values<'a>(&'a self, b: &mut Separated<'_, 'a, Postgres, &str>) {
                $( b.push_bind(&self.$field); )*
                b.push_bind(self.p_cond_6m).push_bind(self.p_base_6m).push_bind(self.$s6)
                 .push_bind(self.p_cond_1y).push_bind(self.p_base_1y).push_bind(self.$s1)
                 .push_bind(self.p_cond_all).push_bind(self.p_base_all).push_bind(self.$sa);
            }
        }
    };
}


// 1. Session Base Rate
impl_persistable!(mm::SessionBaseRateML, 
    "INSERT INTO session_base_rates_ml (asset_id, cs_name, cs_bias, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, cs_name, cs_bias) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, session_name, session_bias);

// 2. Bar Base Rate
impl_persistable!(mm::BarBaseRateML,
    "INSERT INTO bar_base_rates_ml (asset_id, bar_num, bar_bias, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, bar_num, bar_bias) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, bar_num, bar_bias);

// 3. Daily Base Rate
impl_persistable!(mm::DailyBaseRateML,
    "INSERT INTO daily_base_rates_ml (asset_id, daily_bias, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, daily_bias) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, daily_bias);

// 4. Transition 2nd Order
impl_persistable!(mm::Transition2ndOrderML,
    "INSERT INTO session_transition_2nd_order_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias);

// 5. Bar Transition 2nd Order
impl_persistable!(mm::BarTransition2ndOrderML,
    "INSERT INTO bar_transition_2nd_order_ml (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias);

// 6. Bar to Daily Outcome
impl_persistable!(mm::BarToDailyOutcomeML,
    "INSERT INTO bar_to_daily_outcome_ml (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type);

// 7. Session to Daily Outcome
impl_persistable!(mm::SessionToDailyOutcomeML,
    "INSERT INTO session_to_daily_outcome_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type);

// 8. Session to Bar Outcome
impl_persistable!(mm::SessionToBarOutcomeML,
    "INSERT INTO session_to_bar_outcome_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias);

// Daily 3rd Order Outcome
impl_persistable!(mm::DailyOutcome3rdOrderML,
    "INSERT INTO daily_outcome_3rd_order_ml (asset_id, pd2_bias, pd1_bias, pd1_dow, day_type, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, pd2_bias, pd1_bias, pd1_dow, day_type) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, pd2_bias, pd1_bias, pd1_dow, day_type);

// 9. TCS Scores
impl_score_persistable!(mm::Tcs2ndOrderML,
    "INSERT INTO session_transition_lift_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, p_cond_6m, p_base_6m, tcs_score_6m, p_cond_1y, p_base_1y, tcs_score_1y, p_cond_all, p_base_all, tcs_score_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias) DO UPDATE SET p_cond_6m=EXCLUDED.p_cond_6m, p_base_6m=EXCLUDED.p_base_6m, tcs_score_6m=EXCLUDED.tcs_score_6m, p_cond_1y=EXCLUDED.p_cond_1y, p_base_1y=EXCLUDED.p_base_1y, tcs_score_1y=EXCLUDED.tcs_score_1y, p_cond_all=EXCLUDED.p_cond_all, p_base_all=EXCLUDED.p_base_all, tcs_score_all=EXCLUDED.tcs_score_all",
    tcs_score_6m, tcs_score_1y, tcs_score_all,
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias);

// 10. STCS Scores
impl_score_persistable!(mm::Stcs2ndOrderML,
    "INSERT INTO stcs_2nd_order_ml (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias, p_cond_6m, p_base_6m, stcs_score_6m, p_cond_1y, p_base_1y, stcs_score_1y, p_cond_all, p_base_all, stcs_score_all) ",
    " ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias) DO UPDATE SET p_cond_6m=EXCLUDED.p_cond_6m, p_base_6m=EXCLUDED.p_base_6m, stcs_score_6m=EXCLUDED.stcs_score_6m, p_cond_1y=EXCLUDED.p_cond_1y, p_base_1y=EXCLUDED.p_base_1y, stcs_score_1y=EXCLUDED.stcs_score_1y, p_cond_all=EXCLUDED.p_cond_all, p_base_all=EXCLUDED.p_base_all, stcs_score_all=EXCLUDED.stcs_score_all",
    stcs_score_6m, stcs_score_1y, stcs_score_all,
    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias);

// 11. Bar to Daily Lift
impl_score_persistable!(mm::BarToDailyLiftML,
    "INSERT INTO bar_to_daily_lift_ml (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type, p_cond_6m, p_base_6m, lift_6m, p_cond_1y, p_base_1y, lift_1y, p_cond_all, p_base_all, lift_all) ",
    " ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type) DO UPDATE SET p_cond_6m=EXCLUDED.p_cond_6m, p_base_6m=EXCLUDED.p_base_6m, lift_6m=EXCLUDED.lift_6m, p_cond_1y=EXCLUDED.p_cond_1y, p_base_1y=EXCLUDED.p_base_1y, lift_1y=EXCLUDED.lift_1y, p_cond_all=EXCLUDED.p_cond_all, p_base_all=EXCLUDED.p_base_all, lift_all=EXCLUDED.lift_all",
    lift_6m, lift_1y, lift_all,
    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type);

// 12. Session to Daily Lift
impl_score_persistable!(mm::SessionToDailyLiftML,
    "INSERT INTO session_to_daily_lift_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type, p_cond_6m, p_base_6m, lift_6m, p_cond_1y, p_base_1y, lift_1y, p_cond_all, p_base_all, lift_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type) DO UPDATE SET p_cond_6m=EXCLUDED.p_cond_6m, p_base_6m=EXCLUDED.p_base_6m, lift_6m=EXCLUDED.lift_6m, p_cond_1y=EXCLUDED.p_cond_1y, p_base_1y=EXCLUDED.p_base_1y, lift_1y=EXCLUDED.lift_1y, p_cond_all=EXCLUDED.p_cond_all, p_base_all=EXCLUDED.p_base_all, lift_all=EXCLUDED.lift_all",
    lift_6m, lift_1y, lift_all,
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type);

// 13. Session to Bar Lift
impl_score_persistable!(mm::SessionToBarLiftML,
    "INSERT INTO session_to_bar_lift_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias, p_cond_6m, p_base_6m, lift_6m, p_cond_1y, p_base_1y, lift_1y, p_cond_all, p_base_all, lift_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias) DO UPDATE SET p_cond_6m=EXCLUDED.p_cond_6m, p_base_6m=EXCLUDED.p_base_6m, lift_6m=EXCLUDED.lift_6m, p_cond_1y=EXCLUDED.p_cond_1y, p_base_1y=EXCLUDED.p_base_1y, lift_1y=EXCLUDED.lift_1y, p_cond_all=EXCLUDED.p_cond_all, p_base_all=EXCLUDED.p_base_all, lift_all=EXCLUDED.lift_all",
    lift_6m, lift_1y, lift_all,
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias);

// Daily 3rd Order Lift
impl_score_persistable!(mm::DailyOutcome3rdOrderLiftML,
    "INSERT INTO daily_outcome_3rd_order_lift_ml (asset_id, pd2_bias, pd1_bias, pd1_dow, day_type, p_cond_6m, p_base_6m, lift_6m, p_cond_1y, p_base_1y, lift_1y, p_cond_all, p_base_all, lift_all) ",
    " ON CONFLICT (asset_id, pd2_bias, pd1_bias, pd1_dow, day_type) DO UPDATE SET p_cond_6m=EXCLUDED.p_cond_6m, p_base_6m=EXCLUDED.p_base_6m, lift_6m=EXCLUDED.lift_6m, p_cond_1y=EXCLUDED.p_cond_1y, p_base_1y=EXCLUDED.p_base_1y, lift_1y=EXCLUDED.lift_1y, p_cond_all=EXCLUDED.p_cond_all, p_base_all=EXCLUDED.p_base_all, lift_all=EXCLUDED.lift_all",
    lift_6m, lift_1y, lift_all,
    asset_id, pd2_bias, pd1_bias, pd1_dow, day_type);

// Session Continuation
impl_persistable!(mm::SessionContinuationML,
    "INSERT INTO session_continuation_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name);

// Bar Continuation
impl_persistable!(mm::BarContinuationML,
    "INSERT INTO bar_continuation_ml (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num);

// Daily Continuation
impl_persistable!(mm::DailyContinuationML,
    "INSERT INTO daily_continuation_ml (asset_id, pd2_bias, pd1_bias, pd1_dow, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, pd2_bias, pd1_bias, pd1_dow) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, pd2_bias, pd1_bias, pd1_dow);

// Session Extreme Base Rate

impl_persistable!(mm::SessionExtremeBaseRateML,
    "INSERT INTO session_extreme_base_rate_ml (asset_id, session_name, extreme_type, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, session_name, extreme_type) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, session_name, extreme_type);

// Session Extreme Outcome

impl_persistable!(mm::SessionExtremeOutcomeML,
    "INSERT INTO session_extreme_outcome_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, extreme_type, extreme_session_name, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, extreme_type, extreme_session_name) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, extreme_type, extreme_session_name);

// Session Extreme Lift
impl_score_persistable!(mm::SessionExtremeLiftML,
    "INSERT INTO session_extreme_lift_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, extreme_type, extreme_session_name, p_cond_6m, p_base_6m, lift_6m, p_cond_1y, p_base_1y, lift_1y, p_cond_all, p_base_all, lift_all) ",
    " ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, extreme_type, extreme_session_name) DO UPDATE SET p_cond_6m=EXCLUDED.p_cond_6m, p_base_6m=EXCLUDED.p_base_6m, lift_6m=EXCLUDED.lift_6m, p_cond_1y=EXCLUDED.p_cond_1y, p_base_1y=EXCLUDED.p_base_1y, lift_1y=EXCLUDED.lift_1y, p_cond_all=EXCLUDED.p_cond_all, p_base_all=EXCLUDED.p_base_all, lift_all=EXCLUDED.lift_all",
    lift_6m, lift_1y, lift_all,
    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, extreme_type, extreme_session_name);
// Bar Extreme Base Rate
impl_persistable!(mm::BarExtremeBaseRateML,
    "INSERT INTO bar_extreme_base_rate_ml (asset_id, cb_num, extreme_type, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, cb_num, extreme_type) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, cb_num, extreme_type);
// Bar Extreme Outcome
impl_persistable!(mm::BarExtremeOutcomeML,
    "INSERT INTO bar_extreme_outcome_ml (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, extreme_type, extreme_cb_num, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) ",
    " ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, extreme_type, extreme_cb_num) DO UPDATE SET p_6m=EXCLUDED.p_6m, count_6m=EXCLUDED.count_6m, total_6m=EXCLUDED.total_6m, p_1y=EXCLUDED.p_1y, count_1y=EXCLUDED.count_1y, total_1y=EXCLUDED.total_1y, p_all=EXCLUDED.p_all, count_all=EXCLUDED.count_all, total_all=EXCLUDED.total_all",
    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, extreme_type, extreme_cb_num);
// Bar Extreme Lift
impl_score_persistable!(mm::BarExtremeLiftML,
    "INSERT INTO bar_extreme_lift_ml (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, extreme_type, extreme_cb_num, p_cond_6m, p_base_6m, lift_6m, p_cond_1y, p_base_1y, lift_1y, p_cond_all, p_base_all, lift_all) ",
    " ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, extreme_type, extreme_cb_num) DO UPDATE SET lift_6m=EXCLUDED.lift_6m, lift_1y=EXCLUDED.lift_1y, lift_all=EXCLUDED.lift_all",
    lift_6m, lift_1y, lift_all,
    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, extreme_type, extreme_cb_num);