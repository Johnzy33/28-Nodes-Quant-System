
use anyhow::{Ok, Result};
use log::{info};
use shared_models::{DayTypeBaseRate, Tcs2ndOrder, Transition2ndOrder, BarDayType2ndOrder,
    BarTransition2ndOrder,Btcs2ndOrder,BpcsScore2ndOrder,
    TcsContinuationScore,DayType2ndOrder,  PcsScore2ndOrder, CsBaseRate, CbBaseRate,
    HighLowSession2ndOrder, DayOutcome3rdOrder,  Dcs3rdOrder, DailyTrendContinuationRate,
    SessionToBarMetrics, SessionToBarStcs,
};

impl super::MetricsService {
    
    // Persist CS Bias Base Rate.
   pub async fn persist_cs_base_rates(
        &self,
        base_rates: Vec<CsBaseRate>,
    ) -> Result<()> {

        if base_rates.is_empty() {
            return Ok(());
        }

        for chunk in base_rates.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO cs_base_rates_ml (asset_id, cs_name, cs_bias, p_base) "
            );
            query_builder.push_values(
                chunk.iter(),
                |mut b, rate| {
                    b.push_bind(rate.asset_id.clone())
                     .push_bind(rate.cs_name.clone())
                     .push_bind(rate.cs_bias.clone())
                     .push_bind(rate.p_base);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, cs_name, cs_bias) DO UPDATE SET
                    p_base = EXCLUDED.p_base
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist CS Base Rates: {}", e))?;
        } 
        info!("Successfully persisted {} CS Base Rates.", base_rates.len());

        Ok(())
    }


    

    pub async fn persist_cb_base_rates(
    &self,
    base_rates: Vec<CbBaseRate>,
) -> Result<()> {
    if base_rates.is_empty() {
        return Ok(());
    }

    for chunk in base_rates.chunks(3000) {
        let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO cb_base_rates_ml (
                asset_id, cb_num, cb_bias, p_base
            ) "
        );

        query_builder.push_values(
            chunk.iter(),
            |mut b, r| {
                b.push_bind(r.asset_id.clone())
                 .push_bind(r.cb_num) // SQLX maps Option<i32> to INTEGER NULL
                 .push_bind(r.cb_bias.clone())
                 .push_bind(r.p_base);
            },
        );

        query_builder.push(r#"
            ON CONFLICT (asset_id, cb_num, cb_bias) DO UPDATE SET
                p_base = EXCLUDED.p_base
        "#);

        let query = query_builder.build();
        query.execute(&self.pool).await
            .map_err(|e| anyhow::anyhow!("Failed to persist CB Base Rates: {}", e))?;
    }

    info!("Successfully persisted {} CB Base Rates.", base_rates.len());
    Ok(())
}

    // Persist Daily Bias Base Rate
    pub async fn persist_daily_base_rates(
        &self,
        base_rates: Vec<DayTypeBaseRate>,
    ) -> Result<()> {

        if base_rates.is_empty() {
            return Ok(());
        }

        for chunk in base_rates.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO day_type_base_rates_ml (asset_id, daily_bias, p_day_type_base) "
            );
            query_builder.push_values(
                chunk.iter(),
                |mut b, rate| {
                    b.push_bind(rate.asset_id.clone())
                     .push_bind(rate.daily_bias.clone())
                     .push_bind(rate.p_day_type_base.clone());
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, daily_bias) DO UPDATE SET
                    p_day_type_base = EXCLUDED.p_day_type_base
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist daily Base Rates: {}", e))?;
        } 
        info!("Successfully persisted {} daily Base Rates.", base_rates.len());

        Ok(())
    }

    // Persist 2nd Order Transitional Condition Calculations
   pub async fn persist_transition_2nd_order(
        &self,
        transitions: Vec<Transition2ndOrder>,
    ) -> Result<()> {

        if transitions.is_empty() {
            return Ok(());
        }

        
        for chunk in transitions.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO transition_2nd_order_ml (
                    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, 
                    reliable_total_attempts, reliable_success_count, p_transition_conditional
                ) "
            );
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.ps2_name.clone())
                     .push_bind(t.ps2_bias.clone())
                     .push_bind(t.ps1_name.clone())
                     .push_bind(t.ps1_bias.clone())
                     .push_bind(t.cs_name.clone())
                     .push_bind(t.cs_bias.clone())
                     .push_bind(t.reliable_total_attempts)
                     .push_bind(t.reliable_success_count)
                     .push_bind(t.p_transition_conditional);
            });

            
            query_builder.push(r#"
                ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias) DO UPDATE SET
                    reliable_total_attempts = EXCLUDED.reliable_total_attempts,
                    reliable_success_count = EXCLUDED.reliable_success_count,
                    p_transition_conditional = EXCLUDED.p_transition_conditional
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist 2nd Order Transitions: {}", e))?;
        } 
        info!("Successfully persisted {} 2nd Order Transitions for.", transitions.len() );

        Ok(())
    }

    pub async fn persist_bar_transition_2nd_order(
        &self,
        transitions: Vec<BarTransition2ndOrder>,
    ) -> Result<()> {

        if transitions.is_empty() {
            return Ok(());
        }

        
        for chunk in transitions.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO bar_transition_2nd_order_ml (
                    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias, 
                    reliable_total_attempts, reliable_success_count, p_transition_conditional
                ) "
            );
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.pb2_num.clone())
                     .push_bind(t.pb2_bias.clone())
                     .push_bind(t.pb1_num.clone())
                     .push_bind(t.pb1_bias.clone())
                     .push_bind(t.cb_num.clone())
                     .push_bind(t.cb_bias.clone())
                     .push_bind(t.reliable_total_attempts)
                     .push_bind(t.reliable_success_count)
                     .push_bind(t.p_transition_conditional);
            });

            
            query_builder.push(r#"
                ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias) DO UPDATE SET
                    reliable_total_attempts = EXCLUDED.reliable_total_attempts,
                    reliable_success_count = EXCLUDED.reliable_success_count,
                    p_transition_conditional = EXCLUDED.p_transition_conditional
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist Bar 2nd Order Transitions: {}", e))?;
        } 
        info!("Successfully persisted {} Bar 2nd Order Transitions for.", transitions.len() );

        Ok(())
    }

    // Presist 2nd Order Day Type Calcuations 
    pub async fn persist_day_type_2nd_order(
        &self,
        transitions: Vec<DayType2ndOrder>,
    ) -> Result<()> {

        if transitions.is_empty() {
            return Ok(());
        }

        for chunk in transitions.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO day_type_2nd_order_ml (
                    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type, 
                    reliable_total_attempts, reliable_success_count, p_day_type_conditional
                ) "
            );
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.ps2_name.clone())
                     .push_bind(t.ps2_bias.clone())
                     .push_bind(t.ps1_name.clone())
                     .push_bind(t.ps1_bias.clone())
                     .push_bind(t.day_type.clone())
                     .push_bind(t.reliable_total_attempts)
                     .push_bind(t.reliable_success_count)
                     .push_bind(t.p_day_type_conditional);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, day_type) DO UPDATE SET
                    reliable_total_attempts = EXCLUDED.reliable_total_attempts,
                    reliable_success_count = EXCLUDED.reliable_success_count,
                    p_day_type_conditional = EXCLUDED.p_day_type_conditional
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist Day Type 2nd Order Transitions: {}", e))?;
        } 
        info!("Successfully persisted {} Day Type 2nd Order Transitions.", transitions.len());

        Ok(())
    }

    pub async fn persist_bar_day_type_2nd_order(
        &self,
        transitions: Vec<BarDayType2ndOrder>,
    ) -> Result<()> {

        if transitions.is_empty() {
            return Ok(());
        }

        for chunk in transitions.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO bar_day_type_2nd_order_ml (
                    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type, 
                    reliable_total_attempts, reliable_success_count, p_day_type_conditional
                ) "
            );
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.pb2_num.clone())
                     .push_bind(t.pb2_bias.clone())
                     .push_bind(t.pb1_num.clone())
                     .push_bind(t.pb1_bias.clone())
                     .push_bind(t.day_type.clone())
                     .push_bind(t.reliable_total_attempts)
                     .push_bind(t.reliable_success_count)
                     .push_bind(t.p_day_type_conditional);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, day_type) DO UPDATE SET
                    reliable_total_attempts = EXCLUDED.reliable_total_attempts,
                    reliable_success_count = EXCLUDED.reliable_success_count,
                    p_day_type_conditional = EXCLUDED.p_day_type_conditional
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist Day Type 2nd Bar Order Transitions: {}", e))?;
        } 
        info!("Successfully persisted {} Day Type 2nd Order Bar Transitions.", transitions.len());

        Ok(())
    }

    // Persist 2nd Order Session-to-Bar Calculations
pub async fn persist_session_to_bar_metrics(
    &self,
    transitions: Vec<SessionToBarMetrics>,
) -> Result<()> {

    if transitions.is_empty() {
        return Ok(());
    }

    // Chunking to stay within Postgres parameter limits
    for chunk in transitions.chunks(2500) {

        let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO session_to_bar_2nd_order_ml (
                asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, 
                target_bar_num, target_bar_bias, 
                total_attempts, success_count, probability
            ) "
        );
        
        query_builder.push_values(
            chunk.iter(),
            |mut b, t| {
                b.push_bind(t.asset_id.clone())
                 .push_bind(t.ps2_name.clone())
                 .push_bind(t.ps2_bias.clone())
                 .push_bind(t.ps1_name.clone())
                 .push_bind(t.ps1_bias.clone())
                 .push_bind(t.target_bar_num)
                 .push_bind(t.target_bar_bias.clone())
                 .push_bind(t.total_attempts)
                 .push_bind(t.success_count)
                 .push_bind(t.probability);
        });

        // The conflict target includes both the session sequence and the specific bar outcome
        query_builder.push(r#"
            ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, target_bar_num, target_bar_bias) 
            DO UPDATE SET
                total_attempts = EXCLUDED.total_attempts,
                success_count = EXCLUDED.success_count,
                probability = EXCLUDED.probability
        "#);

        let query = query_builder.build();
        query.execute(&self.pool).await
            .map_err(|e| anyhow::anyhow!("Failed to persist Session to Bar Transitions: {}", e))?;
    } 

    info!("Successfully persisted {} Session-to-Bar 2nd Order Transitions.", transitions.len());

    Ok(())
}

    // Persist TCS Score Calculations
    pub async fn persist_tcs_scores(
        &self,
        tcs_scores: Vec<Tcs2ndOrder>,
    ) -> Result<()> {

        if tcs_scores.is_empty() {
            return Ok(());
        }

        for chunk in tcs_scores.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO tcs_2nd_order_ml (
                    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias, 
                    p_transition_conditional, p_cs_base, tcs_score
                ) "
            );
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.ps2_name.clone())
                     .push_bind(t.ps2_bias.clone())
                     .push_bind(t.ps1_name.clone())
                     .push_bind(t.ps1_bias.clone())
                     .push_bind(t.cs_name.clone())
                     .push_bind(t.cs_bias.clone())
                     .push_bind(t.p_transition_conditional)
                     .push_bind(t.p_cs_base)
                     .push_bind(t.tcs_score);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, cs_name, cs_bias) DO UPDATE SET
                    p_transition_conditional = EXCLUDED.p_transition_conditional,
                    p_cs_base = EXCLUDED.p_cs_base,
                    tcs_score = EXCLUDED.tcs_score
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist TCS Scores: {}", e))?;
        } 
        info!("Successfully persisted {} TCS Scores.", tcs_scores.len());

        Ok(())
    }

     pub async fn persist_btcs_scores(
        &self,
        btcs_scores: Vec<Btcs2ndOrder>,
    ) -> Result<()> {

        if btcs_scores.is_empty() {
            return Ok(());
        }

        for chunk in btcs_scores.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO btcs_2nd_order_ml (
                    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias, 
                    p_transition_conditional, p_cb_base, btcs_score
                ) "
            );
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.pb2_num.clone())
                     .push_bind(t.pb2_bias.clone())
                     .push_bind(t.pb1_num.clone())
                     .push_bind(t.pb1_bias.clone())
                     .push_bind(t.cb_num.clone())
                     .push_bind(t.cb_bias.clone())
                     .push_bind(t.p_transition_conditional)
                     .push_bind(t.p_cb_base)
                     .push_bind(t.btcs_score);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, cb_num, cb_bias) DO UPDATE SET
                    p_transition_conditional = EXCLUDED.p_transition_conditional,
                    p_cb_base = EXCLUDED.p_cb_base,
                    btcs_score = EXCLUDED.btcs_score
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist BTCS Scores: {}", e))?;
        } 
        info!("Successfully persisted {} BTCS Scores.", btcs_scores.len());

        Ok(())
    }

    // Persist Session-to-Bar STCS Scores
pub async fn persist_stcs_scores(
    &self,
    scores: Vec<SessionToBarStcs>,
) -> Result<()> {

    if scores.is_empty() {
        return Ok(());
    }

    for chunk in scores.chunks(2500) {
        let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO session_to_bar_stcs_ml (
                asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, 
                target_bar_num, target_bar_bias, 
                p_conditional, p_bar_base, stcs_score
            ) "
        );
        
        query_builder.push_values(
            chunk.iter(),
            |mut b, s| {
                b.push_bind(s.asset_id.clone())
                 .push_bind(s.ps2_name.clone())
                 .push_bind(s.ps2_bias.clone())
                 .push_bind(s.ps1_name.clone())
                 .push_bind(s.ps1_bias.clone())
                 .push_bind(s.target_bar_num)
                 .push_bind(s.target_bar_bias.clone())
                 .push_bind(s.probability)
                 .push_bind(s.p_bar_base)
                 .push_bind(s.stcs_score);
        });

        query_builder.push(r#"
            ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, target_bar_num, target_bar_bias) 
            DO UPDATE SET
                p_conditional = EXCLUDED.p_conditional,
                p_bar_base = EXCLUDED.p_bar_base,
                stcs_score = EXCLUDED.stcs_score
        "#);

        let query = query_builder.build();
        query.execute(&self.pool).await
            .map_err(|e| anyhow::anyhow!("Failed to persist STCS Scores: {}", e))?;
    } 

    info!("Successfully persisted {} Session-to-Bar STCS scores.", scores.len());

    Ok(())
}
    
    
    // Presist PCS Calculations
    pub async fn persist_pcs_scores(
        &self,
        pcs_scores: Vec<PcsScore2ndOrder>,
    ) -> Result<()> {

        if pcs_scores.is_empty() {
            return Ok(());
        }

        for chunk in pcs_scores.chunks(3000) {

            // Define the INSERT target columns for the tcs_continuation_score table
            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO predictive_confidence_score_2nd_order_ml (
                    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, daily_bias, 
                    p_day_type_conditional, p_day_type_base, pcs_score
                ) "
            );
    
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.ps2_name.clone())
                     .push_bind(t.ps2_bias.clone())
                     .push_bind(t.ps1_name.clone())
                     .push_bind(t.ps1_bias.clone())
                     .push_bind(t.daily_bias.clone())
                     .push_bind(t.p_day_type_conditional)
                     .push_bind(t.p_day_type_base)
                     .push_bind(t.pcs_score);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, daily_bias ) DO UPDATE SET
                    p_day_type_conditional = EXCLUDED.p_day_type_conditional,
                    p_day_type_base = EXCLUDED.p_day_type_base,
                    pcs_score = EXCLUDED.pcs_score
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist PCS Scores: {}", e))?;
        } 
        info!("Successfully persisted {} PCS Scores.", pcs_scores.len());

        Ok(())
    }

    pub async fn persist_bpcs_scores(
        &self,
        bpcs_scores: Vec<BpcsScore2ndOrder>,
    ) -> Result<()> {

        if bpcs_scores.is_empty() {
            return Ok(());
        }

        for chunk in bpcs_scores.chunks(3000) {

            // Define the INSERT target columns for the tcs_continuation_score table
            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO bar_predictive_confidence_score_2nd_order_ml (
                    asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, daily_bias, 
                    p_day_type_conditional, p_day_type_base, bpcs_score
                ) "
            );
    
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.pb2_num.clone())
                     .push_bind(t.pb2_bias.clone())
                     .push_bind(t.pb1_num.clone())
                     .push_bind(t.pb1_bias.clone())
                     .push_bind(t.daily_bias.clone())
                     .push_bind(t.p_day_type_conditional)
                     .push_bind(t.p_day_type_base)
                     .push_bind(t.pcs_score);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, pb2_num, pb2_bias, pb1_num, pb1_bias, daily_bias ) DO UPDATE SET
                    p_day_type_conditional = EXCLUDED.p_day_type_conditional,
                    p_day_type_base = EXCLUDED.p_day_type_base,
                    bpcs_score = EXCLUDED.bpcs_score
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist BPCS Scores: {}", e))?;
        } 
        info!("Successfully persisted {} BPCS Scores.", bpcs_scores.len());

        Ok(())
    }

    


    // Persist TCS Continuation Scores Calculation
    pub async fn persist_tcs_continuation_scores(
        &self,
        tcs_continuation_scores: Vec<TcsContinuationScore>,
    ) -> Result<()> {

        if tcs_continuation_scores.is_empty() {
            return Ok(());
        }

        for chunk in tcs_continuation_scores.chunks(3000) {

        
            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO tcs_continuation_score_ml (
                    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, 
                    p_continuation
                ) "
            ); 
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.ps2_name.clone())
                     .push_bind(t.ps2_bias.clone())
                     .push_bind(t.ps1_name.clone())
                     .push_bind(t.ps1_bias.clone())
                     .push_bind(t.p_continuation);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias) DO UPDATE SET
                    p_continuation = EXCLUDED.p_continuation
                   
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist TCS Continuation Scores: {}", e))?;
        } 
        info!("Successfully persisted {} TCS Continuation Scores.", tcs_continuation_scores.len());

        Ok(())
    }

    // Presist 2nd Order Session Extremes Calculations 
    pub async fn persist_high_low_session_2nd_order(
        &self,
        extremes: Vec<HighLowSession2ndOrder>,
    ) -> Result<()> {

        if extremes.is_empty() {
            return Ok(());
        }

        for chunk in extremes.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO high_low_session_2nd_order_ml (
                    asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, session_extreme, 
                    extreme_session_name, reliable_total_attempts, reliable_success_count, 
                    p_extreme_session_conditional
                ) "
            );
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, t| {
                    b.push_bind(t.asset_id.clone())
                     .push_bind(t.ps2_name.clone())
                     .push_bind(t.ps2_bias.clone())
                     .push_bind(t.ps1_name.clone())
                     .push_bind(t.ps1_bias.clone())
                     .push_bind(t.session_extreme.clone())
                     .push_bind(t.extreme_session_name.clone())
                     .push_bind(t.reliable_total_attempts)
                     .push_bind(t.reliable_success_count)
                     .push_bind(t.p_extreme_session_conditional);
            });
   
            query_builder.push(r#"
                ON CONFLICT (asset_id, ps2_name, ps2_bias, ps1_name, ps1_bias, session_extreme, extreme_session_name) DO UPDATE SET
                    reliable_total_attempts = EXCLUDED.reliable_total_attempts,
                    reliable_success_count = EXCLUDED.reliable_success_count,
                    p_extreme_session_conditional = EXCLUDED.p_extreme_session_conditional
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist High Low Session 2nd Order: {}", e))?;
        } 
        info!("Successfully persisted {} High Low Session 2nd Order records.", extremes.len());

        Ok(())
    }


        // --- Daily Metrics Persist Begins Here  ----

    pub async fn persist_daily_3rd_order_conditional(
        &self,
        scores: Vec<DayOutcome3rdOrder>,
    ) -> Result<()> {

        if scores.is_empty() {
            return Ok(());
        }

        for chunk in scores.chunks(3000) {

            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO day_outcome_3rd_order_ml (
                    asset_id, pd2_bias, pd1_bias, pd1_dow, c_day_bias, 
                    reliable_total_attempts, reliable_success_count, 
                    p_day_type_conditional
                ) "
            );
            
            // Generate the VALUES clause
            query_builder.push_values(
                chunk.iter(),
                |mut b, s| {
                    b.push_bind(s.asset_id.clone())
                     .push_bind(s.pd2_bias.clone())
                     .push_bind(s.pd1_bias.clone())
                     .push_bind(s.pd1_dow.clone())
                     .push_bind(s.c_day_bias.clone())
                     .push_bind(s.reliable_total_attempts)
                     .push_bind(s.reliable_success_count)
                     .push_bind(s.p_day_bias_conditional);
                     
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, pd2_bias, pd1_bias, pd1_dow, c_day_bias) DO UPDATE SET
                    reliable_total_attempts = EXCLUDED.reliable_total_attempts,
                    reliable_success_count = EXCLUDED.reliable_success_count,
                    p_day_type_conditional = EXCLUDED.p_day_type_conditional
                   
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist 3rd Order Day type Scores: {}", e))?;
        } 
        info!("Successfully persisted {} Day Bias 3rd Order.", scores.len());

        Ok(())
    }

    pub async fn persist_dcs_scores(
        &self,
        dcs_scores: Vec<Dcs3rdOrder>,
    ) -> Result<()> {

        if dcs_scores.is_empty() {
            return Ok(());
        }

        for chunk in dcs_scores.chunks(3000) {

            
            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO daily_confidence_score_ml (
                    asset_id, pd2_bias, pd1_bias, pd1_dow, day_bias,  
                    p_day_type_conditional, p_day_type_base, dcs_score
                ) "
            );
            
            query_builder.push_values(
                chunk.iter(),
                |mut b, s| {
                    b.push_bind(s.asset_id.clone())
                     .push_bind(s.pd2_bias.clone())
                     .push_bind(s.pd1_bias.clone())
                     .push_bind(s.pd1_dow.clone())
                     .push_bind(s.c_day_bias.clone())
                     .push_bind(s.p_day_bias_conditional)
                     .push_bind(s.p_day_type_base)
                     .push_bind(s.dcs_score);
            });

            query_builder.push(r#"
                ON CONFLICT (asset_id, pd2_bias, pd1_bias, pd1_dow, day_bias) DO UPDATE SET
                    p_day_type_conditional = EXCLUDED.p_day_type_conditional,
                    p_day_type_base = EXCLUDED.p_day_type_base,
                    dcs_score = EXCLUDED.dcs_score
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist DCS Scores: {}", e))?;
        } 
        info!("Successfully persisted {} DCS Scores.", dcs_scores.len());

        Ok(())
    }


    pub async fn persist_daily_trend_continuation_rate(
        &self,
        rates: Vec<DailyTrendContinuationRate>,
    ) -> Result<()> {

        if rates.is_empty() {
            return Ok(());
        }

        for chunk in rates.chunks(3000) {

            // Define the INSERT target columns
            let mut query_builder: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
                "INSERT INTO daily_trend_continuation_rate (
                    asset_id, pd2_bias, pd1_bias, pd1_dow, c_day_bias,
                    reliable_total_attempts, reliable_success_count, 
                    p_continuation_conditional
                ) "
            );
            
            // Generate the VALUES clause
            query_builder.push_values(
                chunk.iter(),
                |mut b, r| {
                    b.push_bind(r.asset_id.clone())
                     .push_bind(r.pd2_bias.clone())
                     .push_bind(r.pd1_bias.clone())
                     .push_bind(r.pd1_dow.clone())
                     .push_bind(r.c_day_bias.clone())
                     .push_bind(r.reliable_total_attempts)
                     .push_bind(r.reliable_success_count)
                     .push_bind(r.p_continuation_conditional);
            });

            // Define the ON CONFLICT clause using the unique key for this binary metric
            // (asset_id, pd2_bias, pd1_bias, pd1_dow)
            query_builder.push(r#"
                ON CONFLICT (asset_id, pd2_bias, pd1_bias, pd1_dow, c_day_bias) DO UPDATE SET
                    reliable_total_attempts = EXCLUDED.reliable_total_attempts,
                    reliable_success_count = EXCLUDED.reliable_success_count,
                    p_continuation_conditional = EXCLUDED.p_continuation_conditional
            "#);

            let query = query_builder.build();
            query.execute(&self.pool).await
                .map_err(|e| anyhow::anyhow!("Failed to persist Daily Trend Continuation Rate: {}", e))?;
        } 
        info!("Successfully persisted {} Daily Trend Continuation Rate records.", rates.len());

        Ok(())
    }
}