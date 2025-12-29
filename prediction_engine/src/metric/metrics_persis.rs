use anyhow::{Ok, Result};
use sqlx::{Postgres, QueryBuilder};
use crate::metric::persistable::PersistableML;
use crate::metrics_service::MetricsService;
use crate::metric::metrics_model as mm;

impl MetricsService {
    async fn bulk_persist<T>(&self, data: Vec<T>, chunk_size: usize) -> Result<()> 
    where T: PersistableML 
    {
        if data.is_empty() { return Ok(()); }

        for chunk in data.chunks(chunk_size) {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(T::insert_sql());

            qb.push_values(chunk, |mut b, item| {
                item.bind_values(&mut b);
            });

            qb.push(T::conflict_sql());

            qb.build().execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn persist_cs_base_rates_ml(&self, r: Vec<mm::SessionBaseRateML>) -> Result<()> {
        self.bulk_persist(r, 3000).await
    }

    pub async fn persist_bar_base_rates_ml(&self, r: Vec<mm::BarBaseRateML>) -> Result<()> {
        self.bulk_persist(r, 3000).await
    }

    pub async fn persist_daily_base_rates_ml(&self, r: Vec<mm::DailyBaseRateML>) -> Result<()> {
        self.bulk_persist(r, 3000).await
    }

    pub async fn persist_transition_2nd_order_ml(&self, r: Vec<mm::Transition2ndOrderML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_tcs_scores_ml(&self, r: Vec<mm::Tcs2ndOrderML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_bar_transition_2nd_order_ml(&self, r: Vec<mm::BarTransition2ndOrderML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_stcs_scores_ml(&self, r: Vec<mm::Stcs2ndOrderML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_bar_to_daily_outcome_ml(&self, r: Vec<mm::BarToDailyOutcomeML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_bar_to_daily_lift_ml(&self, r: Vec<mm::BarToDailyLiftML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_session_to_daily_outcome_ml(&self, r: Vec<mm::SessionToDailyOutcomeML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_session_to_daily_lift_ml(&self, r: Vec<mm::SessionToDailyLiftML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_session_to_bar_outcome_ml(&self, r: Vec<mm::SessionToBarOutcomeML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_session_to_bar_lift_ml(&self, r: Vec<mm::SessionToBarLiftML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_daily_3rd_order_outcome_ml(&self, r: Vec<mm::DailyOutcome3rdOrderML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_daily_3rd_order_lift_ml(&self, r: Vec<mm::DailyOutcome3rdOrderLiftML>) -> Result<()> {
    self.bulk_persist(r, 2000).await
    }

    pub async fn persist_session_continuation_ml(&self, r: Vec<mm::SessionContinuationML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_bar_continuation_ml(&self, r: Vec<mm::BarContinuationML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_daily_continuation_ml(&self, r: Vec<mm::DailyContinuationML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_session_extreme_base_rate_ml(&self, r: Vec<mm::SessionExtremeBaseRateML>) -> Result<()> { 
        self.bulk_persist(r, 2000).await 
    }
    pub async fn persist_session_extreme_outcome_ml(&self, r: Vec<mm::SessionExtremeOutcomeML>) -> Result<()> { 
        self.bulk_persist(r, 2000).await 
    }
    pub async fn persist_session_extreme_lift_ml(&self, r: Vec<mm::SessionExtremeLiftML>) -> Result<()> { 
        self.bulk_persist(r, 2000).await 
    }

    pub async fn persist_bar_extreme_base_rate_ml(&self, r: Vec<mm::BarExtremeBaseRateML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_bar_extreme_outcome_ml(&self, r: Vec<mm::BarExtremeOutcomeML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

    pub async fn persist_bar_extreme_lift_ml(&self, r: Vec<mm::BarExtremeLiftML>) -> Result<()> {
        self.bulk_persist(r, 2000).await
    }

}