
// use anyhow::{Ok, Result};
// use log::{info};
// use crate::metrics_service::MetricsService;
// use crate::metric::metrics_model::{SessionBaseRateML, BarBaseRateML, DailyBaseRateML,
//     Transition2ndOrderML, Tcs2ndOrderML, BarTransition2ndOrderML, Stcs2ndOrderML, BarToDailyOutcomeML, BarToDailyLiftML,
//     SessionToDailyOutcomeML, SessionToDailyLiftML, SessionToBarOutcomeML, SessionToBarLiftML
// };





// impl MetricsService {

//     pub async fn persist_cs_base_rates_ml(
//         &self,
//         base_rates: Vec<SessionBaseRateML>,
//     ) -> Result<()> {
//         if base_rates.is_empty() {
//             return Ok(());
//         }

//         // Process in chunks of 3000 to avoid PostgreSQL parameter limits (65,535)
//         for chunk in base_rates.chunks(3000) {
//             let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO session_base_rates_ml (
//                     asset_id, cs_name, cs_bias, 
//                     p_6m, count_6m, total_6m, 
//                     p_1y, count_1y, total_1y, 
//                     p_all, count_all, total_all
//                 ) "
//             );

//             query_builder.push_values(chunk.iter(), |mut b, rate| {
//                 b.push_bind(rate.asset_id.clone())
//                 .push_bind(rate.session_name.clone())
//                 .push_bind(rate.session_bias.clone())
//                 .push_bind(rate.p_6m)
//                 .push_bind(rate.count_6m)
//                 .push_bind(rate.total_6m)
//                 .push_bind(rate.p_1y)
//                 .push_bind(rate.count_1y)
//                 .push_bind(rate.total_1y)
//                 .push_bind(rate.p_all)
//                 .push_bind(rate.count_all)
//                 .push_bind(rate.total_all);
//             });

//             query_builder.push(r#"
//                 ON CONFLICT (asset_id, cs_name, cs_bias) DO UPDATE SET
//                     p_6m = EXCLUDED.p_6m,
//                     count_6m = EXCLUDED.count_6m,
//                     total_6m = EXCLUDED.total_6m,
//                     p_1y = EXCLUDED.p_1y,
//                     count_1y = EXCLUDED.count_1y,
//                     total_1y = EXCLUDED.total_1y,
//                     p_all = EXCLUDED.p_all,
//                     count_all = EXCLUDED.count_all,
//                     total_all = EXCLUDED.total_all
//             "#);

//             let query = query_builder.build();
//             query.execute(&self.pool).await
//                 .map_err(|e| anyhow::anyhow!("Failed to bulk persist CS Base Rates ML: {}", e))?;
//         } 

//         info!("Successfully persisted {} wide CS Base Rate features.", base_rates.len());
//         Ok(())
//     }

//     pub async fn persist_bar_base_rates_ml(
//         &self,
//         base_rates: Vec<BarBaseRateML>,
//     ) -> Result<()> {
//         if base_rates.is_empty() { return Ok(()); }

//         for chunk in base_rates.chunks(3000) {
//             let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO bar_base_rates_ml (asset_id, bar_num, bar_bias, 
//                     p_6m, count_6m, total_6m, 
//                     p_1y, count_1y, total_1y, 
//                     p_all, count_all, total_all
//                 ) "
//             );

//             query_builder.push_values(chunk.iter(), |mut b, rate| {
//                 b.push_bind(rate.asset_id.clone())
//                 .push_bind(rate.bar_num)
//                 .push_bind(rate.bar_bias.clone())
//                 .push_bind(rate.p_6m).push_bind(rate.count_6m).push_bind(rate.total_6m)
//                 .push_bind(rate.p_1y).push_bind(rate.count_1y).push_bind(rate.total_1y)
//                 .push_bind(rate.p_all).push_bind(rate.count_all).push_bind(rate.total_all);
//             });

//             query_builder.push(" ON CONFLICT (asset_id, bar_num, bar_bias) DO UPDATE SET 
//                 p_6m = EXCLUDED.p_6m, count_6m = EXCLUDED.count_6m, total_6m = EXCLUDED.total_6m,
//                 p_1y = EXCLUDED.p_1y, count_1y = EXCLUDED.count_1y, total_1y = EXCLUDED.total_1y,
//                 p_all = EXCLUDED.p_all, count_all = EXCLUDED.count_all, total_all = EXCLUDED.total_all");

//             query_builder.build().execute(&self.pool).await?;
//         }

//         info!("Successfully persisted {} wide Bar Base Rate features.", base_rates.len());
//         Ok(())
//     }

//     pub async fn persist_daily_base_rates_ml(
//         &self,
//         base_rates: Vec<DailyBaseRateML>,
//     ) -> Result<()> {
//         if base_rates.is_empty() { return Ok(()); }

//         for chunk in base_rates.chunks(3000) {
//             let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO daily_base_rates_ml (asset_id, daily_bias, 
//                     p_6m, count_6m, total_6m, 
//                     p_1y, count_1y, total_1y, 
//                     p_all, count_all, total_all
//                 ) "
//             );

//             query_builder.push_values(chunk.iter(), |mut b, rate| {
//                 b.push_bind(rate.asset_id.clone())
//                 .push_bind(rate.daily_bias.clone())
//                 .push_bind(rate.p_6m).push_bind(rate.count_6m).push_bind(rate.total_6m)
//                 .push_bind(rate.p_1y).push_bind(rate.count_1y).push_bind(rate.total_1y)
//                 .push_bind(rate.p_all).push_bind(rate.count_all).push_bind(rate.total_all);
//             });

//             query_builder.push(" ON CONFLICT (asset_id, daily_bias) DO UPDATE SET 
//                 p_6m = EXCLUDED.p_6m, count_6m = EXCLUDED.count_6m, total_6m = EXCLUDED.total_6m,
//                 p_1y = EXCLUDED.p_1y, count_1y = EXCLUDED.count_1y, total_1y = EXCLUDED.total_1y,
//                 p_all = EXCLUDED.p_all, count_all = EXCLUDED.count_all, total_all = EXCLUDED.total_all");

//             query_builder.build().execute(&self.pool).await?;
//         }

//         info!("Successfully persisted {} wide Daily Base Rate features.", base_rates.len());
//         Ok(())
//     }

//     pub async fn persist_transition_2nd_order_ml(&self, results: Vec<Transition2ndOrderML>) -> Result<()> {
//     if results.is_empty() { return Ok(()); }

//     for chunk in results.chunks(2000) {
//         let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//             "INSERT INTO transition_2nd_order_ml (
//                 asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, 
//                 p_6m, count_6m, total_6m, 
//                 p_1y, count_1y, total_1y, 
//                 p_all, count_all, total_all
//             ) "
//         );

//         query_builder.push_values(chunk.iter(), |mut b, r| {
//             b.push_bind(r.asset_id.clone()).push_bind(r.ps2_name.clone()).push_bind(r.ps2_bias.clone())
//              .push_bind(r.ps1_name.clone()).push_bind(r.ps1_bias.clone()).push_bind(r.cs_name.clone()).push_bind(r.cs_bias.clone())
//              .push_bind(r.p_6m).push_bind(r.count_6m).push_bind(r.total_6m)
//              .push_bind(r.p_1y).push_bind(r.count_1y).push_bind(r.total_1y)
//              .push_bind(r.p_all).push_bind(r.count_all).push_bind(r.total_all);
//         });

//         query_builder.push(" ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias) DO UPDATE SET 
//             p_6m = EXCLUDED.p_6m, count_6m = EXCLUDED.count_6m, total_6m = EXCLUDED.total_6m,
//             p_1y = EXCLUDED.p_1y, count_1y = EXCLUDED.count_1y, total_1y = EXCLUDED.total_1y,
//             p_all = EXCLUDED.p_all, count_all = EXCLUDED.count_all, total_all = EXCLUDED.total_all");

//         query_builder.build().execute(&self.pool).await?;
//     }
//     Ok(())
//     }

//     pub async fn persist_tcs_scores_ml(&self, scores: Vec<Tcs2ndOrderML>) -> Result<()> {
//     for chunk in scores.chunks(2000) {
//         let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//             "INSERT INTO tcs_2nd_order_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, 
//             p_cond_6m, p_base_6m, tcs_score_6m, 
//             p_cond_1y, p_base_1y, tcs_score_1y,
//             p_cond_all, p_base_all, tcs_score_all
//             ) "
//         );
//         qb.push_values(chunk, |mut b, s| {
//             b.push_bind(&s.asset_id).push_bind(&s.ps2_name).push_bind(&s.ps2_bias)
//              .push_bind(&s.ps1_name).push_bind(&s.ps1_bias).push_bind(&s.cs_name).push_bind(&s.cs_bias)
//              .push_bind(s.p_cond_6m).push_bind(s.p_base_6m).push_bind(s.tcs_score_6m)
//              .push_bind(s.p_cond_1y).push_bind(s.p_base_1y).push_bind(s.tcs_score_1y)
//              .push_bind(s.p_cond_all).push_bind(s.p_base_all).push_bind(s.tcs_score_all);
//         });
//         qb.push(" ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias) DO UPDATE SET 
//             p_cond_6m = EXCLUDED.p_cond_6m, p_base_6m = EXCLUDED.p_base_6m, tcs_score_6m = EXCLUDED.tcs_score_6m,
//             p_cond_1y = EXCLUDED.p_cond_1y, p_base_1y = EXCLUDED.p_base_1y, tcs_score_1y = EXCLUDED.tcs_score_1y,
//             p_cond_all = EXCLUDED.p_cond_all, p_base_all = EXCLUDED.p_base_all, tcs_score_all = EXCLUDED.tcs_score_all");
//         qb.build().execute(&self.pool).await?;
//     }
//     Ok(())
//     }
//     pub async fn persist_bar_transition_2nd_order_ml(
//         &self,
//         transitions: Vec<BarTransition2ndOrderML>,
//     ) -> Result<()> {
//         if transitions.is_empty() {
//             return Ok(());
//         }

//         // Process in chunks to respect PostgreSQL parameter limits
//         for chunk in transitions.chunks(2000) {
//             let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO bar_transition_2nd_order_ml (
//                     asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias, 
//                     p_6m, count_6m, total_6m, 
//                     p_1y, count_1y, total_1y, 
//                     p_all, count_all, total_all
//                 ) "
//             );

//             query_builder.push_values(chunk.iter(), |mut b, tr| {
//                 b.push_bind(tr.asset_id.clone())
//                 .push_bind(tr.pb2_num)
//                 .push_bind(tr.pb2_bias.clone())
//                 .push_bind(tr.pb1_num)
//                 .push_bind(tr.pb1_bias.clone())
//                 .push_bind(tr.cb_num)
//                 .push_bind(tr.cb_bias.clone())
//                 .push_bind(tr.p_6m)
//                 .push_bind(tr.count_6m)
//                 .push_bind(tr.total_6m)
//                 .push_bind(tr.p_1y)
//                 .push_bind(tr.count_1y)
//                 .push_bind(tr.total_1y)
//                 .push_bind(tr.p_all)
//                 .push_bind(tr.count_all)
//                 .push_bind(tr.total_all);
//             });

//             query_builder.push(r#"
//                 ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias) 
//                 DO UPDATE SET
//                     p_6m = EXCLUDED.p_6m,
//                     count_6m = EXCLUDED.count_6m,
//                     total_6m = EXCLUDED.total_6m,
//                     p_1y = EXCLUDED.p_1y,
//                     count_1y = EXCLUDED.count_1y,
//                     total_1y = EXCLUDED.total_1y,
//                     p_all = EXCLUDED.p_all,
//                     count_all = EXCLUDED.count_all,
//                     total_all = EXCLUDED.total_all
//             "#);

//             let query = query_builder.build();
//             query.execute(&self.pool).await
//                 .map_err(|e| anyhow::anyhow!("Failed to persist Bar Transitions ML: {}", e))?;
//         } 

//         info!("Successfully persisted {} Bar Transition features.", transitions.len());
//         Ok(())
//     }

//     pub async fn persist_stcs_scores_ml(&self, scores: Vec<Stcs2ndOrderML>) -> Result<()> {
//         for chunk in scores.chunks(2000) {
//             let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO stcs_2nd_order_ml (
//                     asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias, 
//                     p_cond_6m, p_base_6m, stcs_score_6m, 
//                     p_cond_1y, p_base_1y, stcs_score_1y,
//                     p_cond_all, p_base_all, stcs_score_all
//                 ) "
//             );
//             qb.push_values(chunk, |mut b, s| {
//                 b.push_bind(&s.asset_id).push_bind(s.pb2_num).push_bind(&s.pb2_bias)
//                 .push_bind(s.pb1_num).push_bind(&s.pb1_bias).push_bind(s.cb_num).push_bind(&s.cb_bias)
//                 .push_bind(s.p_cond_6m).push_bind(s.p_base_6m).push_bind(s.stcs_score_6m)
//                 .push_bind(s.p_cond_1y).push_bind(s.p_base_1y).push_bind(s.stcs_score_1y)
//                 .push_bind(s.p_cond_all).push_bind(s.p_base_all).push_bind(s.stcs_score_all);
//             });
//             qb.push(" ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias) DO UPDATE SET 
//                 p_cond_6m = EXCLUDED.p_cond_6m, p_base_6m = EXCLUDED.p_base_6m, stcs_score_6m = EXCLUDED.stcs_score_6m,
//                 p_cond_1y = EXCLUDED.p_cond_1y, p_base_1y = EXCLUDED.p_base_1y, stcs_score_1y = EXCLUDED.stcs_score_1y,
//                 p_cond_all = EXCLUDED.p_cond_all, p_base_all = EXCLUDED.p_base_all, stcs_score_all = EXCLUDED.stcs_score_all");
//             qb.build().execute(&self.pool).await?;
//         }
//         Ok(())
//     }

//     pub async fn persist_bar_to_daily_outcome_ml(&self, data: Vec<BarToDailyOutcomeML>) -> Result<()> {
//         for chunk in data.chunks(2000) {
//             let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO bar_to_daily_outcome_ml (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) "
//             );
//             qb.push_values(chunk, |mut b, r| {
//                 b.push_bind(&r.asset_id).push_bind(r.pb2_num).push_bind(&r.pb2_bias).push_bind(r.pb1_num).push_bind(&r.pb1_bias).push_bind(&r.day_type)
//                 .push_bind(r.p_6m).push_bind(r.count_6m).push_bind(r.total_6m)
//                 .push_bind(r.p_1y).push_bind(r.count_1y).push_bind(r.total_1y)
//                 .push_bind(r.p_all).push_bind(r.count_all).push_bind(r.total_all);
//             });
//             qb.push(" ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type) DO UPDATE SET 
//                 p_6m = EXCLUDED.p_6m, count_6m = EXCLUDED.count_6m, total_6m = EXCLUDED.total_6m,
//                 p_1y = EXCLUDED.p_1y, count_1y = EXCLUDED.count_1y, total_1y = EXCLUDED.total_1y,
//                 p_all = EXCLUDED.p_all, count_all = EXCLUDED.count_all, total_all = EXCLUDED.total_all");
//             qb.build().execute(&self.pool).await?;
//         }
//         Ok(())
//     }

//     pub async fn persist_bar_to_daily_lift_ml(&self, lifts: Vec<BarToDailyLiftML>) -> Result<()> {
//         if lifts.is_empty() { return Ok(()); }

//         for chunk in lifts.chunks(2000) {
//             let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO bar_to_daily_lift_ml (
//                     asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type, 
//                     p_cond_6m, p_base_6m, lift_6m, 
//                     p_cond_1y, p_base_1y, lift_1y, 
//                     p_cond_all, p_base_all, lift_all
//                 ) "
//             );

//             qb.push_values(chunk, |mut b, s| {
//                 b.push_bind(&s.asset_id).push_bind(s.pb2_num).push_bind(&s.pb2_bias)
//                 .push_bind(s.pb1_num).push_bind(&s.pb1_bias).push_bind(&s.day_type)
//                 .push_bind(s.p_cond_6m).push_bind(s.p_base_6m).push_bind(s.lift_6m)
//                 .push_bind(s.p_cond_1y).push_bind(s.p_base_1y).push_bind(s.lift_1y)
//                 .push_bind(s.p_cond_all).push_bind(s.p_base_all).push_bind(s.lift_all);
//             });

//             qb.push(" ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type) DO UPDATE SET 
//                 p_cond_6m = EXCLUDED.p_cond_6m, p_base_6m = EXCLUDED.p_base_6m, lift_6m = EXCLUDED.lift_6m,
//                 p_cond_1y = EXCLUDED.p_cond_1y, p_base_1y = EXCLUDED.p_base_1y, lift_1y = EXCLUDED.lift_1y,
//                 p_cond_all = EXCLUDED.p_cond_all, p_base_all = EXCLUDED.p_base_all, lift_all = EXCLUDED.lift_all");

//             qb.build().execute(&self.pool).await?;
//         }
//         Ok(())
//     }

//     pub async fn persist_session_to_daily_outcome_ml(
//         &self, 
//         data: Vec<SessionToDailyOutcomeML>
//     ) -> Result<()> {
//         if data.is_empty() { return Ok(()); }

//         for chunk in data.chunks(2000) {
//             let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO session_to_daily_outcome_ml (
//                     asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type, 
//                     p_6m, count_6m, total_6m, 
//                     p_1y, count_1y, total_1y, 
//                     p_all, count_all, total_all
//                 ) "
//             );

//             qb.push_values(chunk, |mut b, r| {
//                 b.push_bind(&r.asset_id)
//                 .push_bind(&r.ps2_name).push_bind(&r.ps2_bias)
//                 .push_bind(&r.ps1_name).push_bind(&r.ps1_bias)
//                 .push_bind(&r.day_type)
//                 .push_bind(r.p_6m).push_bind(r.count_6m).push_bind(r.total_6m)
//                 .push_bind(r.p_1y).push_bind(r.count_1y).push_bind(r.total_1y)
//                 .push_bind(r.p_all).push_bind(r.count_all).push_bind(r.total_all);
//             });

//             qb.push(r#"
//                 ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type) 
//                 DO UPDATE SET 
//                     p_6m = EXCLUDED.p_6m, count_6m = EXCLUDED.count_6m, total_6m = EXCLUDED.total_6m,
//                     p_1y = EXCLUDED.p_1y, count_1y = EXCLUDED.count_1y, total_1y = EXCLUDED.total_1y,
//                     p_all = EXCLUDED.p_all, count_all = EXCLUDED.count_all, total_all = EXCLUDED.total_all
//             "#);

//             qb.build().execute(&self.pool).await?;
//         }
//         info!("Persisted {} Session-to-Daily Outcome records.", data.len());
//         Ok(())
//     }


//     pub async fn persist_session_to_daily_lift_ml(&self, lifts: Vec<SessionToDailyLiftML>) -> Result<()> {
//         for chunk in lifts.chunks(2000) {
//             let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO session_to_daily_lift_ml (
//                     asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type, 
//                     p_cond_6m, p_base_6m, lift_6m, 
//                     p_cond_1y, p_base_1y, lift_1y, 
//                     p_cond_all, p_base_all, lift_all
//                 ) "
//             );

//             qb.push_values(chunk, |mut b, s| {
//                 b.push_bind(&s.asset_id).push_bind(&s.ps2_name).push_bind(&s.ps2_bias)
//                 .push_bind(&s.ps1_name).push_bind(&s.ps1_bias).push_bind(&s.day_type)
//                 .push_bind(s.p_cond_6m).push_bind(s.p_base_6m).push_bind(s.lift_6m)
//                 .push_bind(s.p_cond_1y).push_bind(s.p_base_1y).push_bind(s.lift_1y)
//                 .push_bind(s.p_cond_all).push_bind(s.p_base_all).push_bind(s.lift_all);
//             });

//             qb.push(" ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type) DO UPDATE SET 
//                 p_cond_6m = EXCLUDED.p_cond_6m, p_base_6m = EXCLUDED.p_base_6m, lift_6m = EXCLUDED.lift_6m,
//                 p_cond_1y = EXCLUDED.p_cond_1y, p_base_1y = EXCLUDED.p_base_1y, lift_1y = EXCLUDED.lift_1y,
//                 p_cond_all = EXCLUDED.p_cond_all, p_base_all = EXCLUDED.p_base_all, lift_all = EXCLUDED.lift_all");

//             qb.build().execute(&self.pool).await?;
//         }
//         Ok(())
//     }

//     pub async fn persist_session_to_bar_outcome_ml(&self, data: Vec<SessionToBarOutcomeML>) -> Result<()> {
//         for chunk in data.chunks(2000) {
//             let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO session_to_bar_outcome_ml (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias, p_6m, count_6m, total_6m, p_1y, count_1y, total_1y, p_all, count_all, total_all) "
//             );
//             qb.push_values(chunk, |mut b, r| {
//                 b.push_bind(&r.asset_id).push_bind(&r.ps2_name).push_bind(&r.ps2_bias).push_bind(&r.ps1_name).push_bind(&r.ps1_bias).push_bind(r.cb_num).push_bind(&r.cb_bias)
//                 .push_bind(r.p_6m).push_bind(r.count_6m).push_bind(r.total_6m)
//                 .push_bind(r.p_1y).push_bind(r.count_1y).push_bind(r.total_1y)
//                 .push_bind(r.p_all).push_bind(r.count_all).push_bind(r.total_all);
//             });
//             qb.push(" ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias) DO UPDATE SET 
//                 p_6m = EXCLUDED.p_6m, count_6m = EXCLUDED.count_6m, total_6m = EXCLUDED.total_6m,
//                 p_1y = EXCLUDED.p_1y, count_1y = EXCLUDED.count_1y, total_1y = EXCLUDED.total_1y,
//                 p_all = EXCLUDED.p_all, count_all = EXCLUDED.count_all, total_all = EXCLUDED.total_all");
//             qb.build().execute(&self.pool).await?;
//         }
//         Ok(())
//     }

//     pub async fn persist_session_to_bar_lift_ml(&self, data: Vec<SessionToBarLiftML>) -> Result<()> {
//         for chunk in data.chunks(2000) {
//             let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
//                 "INSERT INTO session_to_bar_lift_ml (
//                     asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias, 
//                     p_cond_6m, p_base_6m, lift_6m, 
//                     p_cond_1y, p_base_1y, lift_1y, 
//                     p_cond_all, p_base_all, lift_all
//                 ) "
//             );
//             qb.push_values(chunk, |mut b, r| {
//                 b.push_bind(&r.asset_id).push_bind(&r.ps2_name).push_bind(&r.ps2_bias)
//                 .push_bind(&r.ps1_name).push_bind(&r.ps1_bias).push_bind(r.cb_num).push_bind(&r.cb_bias)
//                 .push_bind(r.p_cond_6m).push_bind(r.p_base_6m).push_bind(r.lift_6m)
//                 .push_bind(r.p_cond_1y).push_bind(r.p_base_1y).push_bind(r.lift_1y)
//                 .push_bind(r.p_cond_all).push_bind(r.p_base_all).push_bind(r.lift_all);
//             });
//             qb.push(" ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cb_num, cb_bias) DO UPDATE SET 
//                 p_cond_6m = EXCLUDED.p_cond_6m, p_base_6m = EXCLUDED.p_base_6m, lift_6m = EXCLUDED.lift_6m,
//                 p_cond_1y = EXCLUDED.p_cond_1y, p_base_1y = EXCLUDED.p_base_1y, lift_1y = EXCLUDED.lift_1y,
//                 p_cond_all = EXCLUDED.p_cond_all, p_base_all = EXCLUDED.p_base_all, lift_all = EXCLUDED.lift_all");
//             qb.build().execute(&self.pool).await?;
//         }
//         Ok(())
//     }
// }