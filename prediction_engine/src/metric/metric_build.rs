use anyhow::{Context, Result};
use log::{info, warn};
use std::collections::HashMap;

// Aliases for consistency
use shared_models as sm;
use crate::metric::metrics_model as mm;
use crate::metrics_service::MetricsService;

/// Internal helper to standardize logging and error context
async fn run_calculation<T, F, Fut>(name: &str, asset_id: &str, f: F) -> Result<Vec<T>>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Vec<T>>>,
{
    info!("Starting {} Calculation for asset: {}", name, asset_id);
    
    let result = f().await.with_context(|| format!("{} Calculation failed for {}", name, asset_id))?;

    if result.is_empty() {
        warn!("No {} calculated for {}.", name, asset_id);
    } else {
        info!("Successfully calculated {} {} for {}.", result.len(), name, asset_id);
    }
    
    Ok(result)
}

// --- BASE RATE CALCULATIONS ---

pub async fn cs_base_rates(
    service: &MetricsService, 
    session_contexts: &[sm::SessionContextData], 
    asset_id: &str
) -> Result<Vec<mm::SessionBaseRateML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("CS Base Rate", asset_id, || async {
        service.calculate_cs_base_rates_ml(session_contexts, &intervals).await
    }).await
}

pub async fn bar_base_rates(
    service: &MetricsService, 
    eight_contexts: &[sm::EightContextData], 
    asset_id: &str
) -> Result<Vec<mm::BarBaseRateML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("Bar Base Rate", asset_id, || async {
        service.calculate_bar_base_rates_ml(eight_contexts, &intervals).await
    }).await
}

pub async fn daily_base_rates(
    service: &MetricsService, 
    daily_contexts: &[sm::DailyContextData], 
    asset_id: &str
) -> Result<Vec<mm::DailyBaseRateML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("Daily Base Rate", asset_id, || async {
        service.calculate_daily_base_rates_ml(daily_contexts, &intervals).await
    }).await
}

// --- TRANSITION & SCORE CALCULATIONS ---

pub async fn transition_2nd_order_rates(
    service: &MetricsService, 
    session_contexts: &[sm::SessionContextData], 
    asset_id: &str
) -> Result<Vec<mm::Transition2ndOrderML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("2nd Order Transition", asset_id, || async {
        service.calculate_transition_2nd_order_ml(session_contexts, &intervals).await
    }).await
}

pub async fn tcs_scores_2nd_order(
    service: &MetricsService,
    transitions: Vec<mm::Transition2ndOrderML>,
    base_rates: Vec<mm::SessionBaseRateML>,
    asset_id: &str,
) -> Result<Vec<mm::Tcs2ndOrderML>> {
    run_calculation("TCS 2nd Order Score", asset_id, || async {
        service.calculate_tcs_scores_ml(transitions, base_rates).await
    }).await
}

pub async fn bar_transition_2nd_order_rates(
    service: &MetricsService, 
    eight_contexts: &[sm::EightContextData], 
    asset_id: &str
) -> Result<Vec<mm::BarTransition2ndOrderML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("Bar 2nd Order Transition", asset_id, || async {
        service.calculate_bar_transition_2nd_order_ml(eight_contexts, &intervals).await
    }).await
}

pub async fn stcs_scores_2nd_order(
    service: &MetricsService,
    bar_transitions: Vec<mm::BarTransition2ndOrderML>,
    bar_base_rates: Vec<mm::BarBaseRateML>,
    asset_id: &str,
) -> Result<Vec<mm::Stcs2ndOrderML>> {
    run_calculation("STCS 2nd Order Score", asset_id, || async {
        service.calculate_stcs_scores_ml(bar_transitions, bar_base_rates).await
    }).await
}

// --- OUTCOME & LIFT CALCULATIONS ---

pub async fn bar_to_daily_outcome_rates(
    service: &MetricsService, 
    eight_contexts: &[sm::EightContextData], 
    daily_outcomes: &[sm::DailyContextData], 
    asset_id: &str
) -> Result<Vec<mm::BarToDailyOutcomeML>> {
    let intervals = service.get_lookback_intervals();
    let daily_map: HashMap<_, _> = daily_outcomes.iter().cloned().map(|d| (d.trading_date, d)).collect();

    run_calculation("Bar to Daily Outcome", asset_id, || async {
        service.calculate_bar_to_daily_ml(eight_contexts, &daily_map, &intervals).await
    }).await
}

pub async fn bar_to_daily_lift_rates(
    service: &MetricsService, 
    transitions: Vec<mm::BarToDailyOutcomeML>,
    daily_base_rates: Vec<mm::DailyBaseRateML>,
    asset_id: &str
) -> Result<Vec<mm::BarToDailyLiftML>> {
    run_calculation("Bar to Daily Lift", asset_id, || async {
        service.calculate_bar_to_daily_lift_ml(transitions, daily_base_rates).await
    }).await
}

pub async fn session_to_daily_outcome_rates(
    service: &MetricsService, 
    session_contexts: &[sm::SessionContextData], 
    daily_outcomes: &[sm::DailyContextData],
    asset_id: &str
) -> Result<Vec<mm::SessionToDailyOutcomeML>> {
    let intervals = service.get_lookback_intervals();
    let daily_map: HashMap<_, _> = daily_outcomes.iter().cloned().map(|d| (d.trading_date, d)).collect();

    run_calculation("Session to Daily Outcome", asset_id, || async {
        service.calculate_session_to_daily_ml(session_contexts, &daily_map, &intervals).await
    }).await
}

pub async fn session_to_daily_lift_rates(
    service: &MetricsService, 
    transitions: Vec<mm::SessionToDailyOutcomeML>,
    daily_base_rates: Vec<mm::DailyBaseRateML>,
    asset_id: &str
) -> Result<Vec<mm::SessionToDailyLiftML>> {
    run_calculation("Session to Daily Lift", asset_id, || async {
        service.calculate_session_to_daily_lift_ml(transitions, daily_base_rates).await
    }).await
}

pub async fn session_to_bar_outcome_rates(
    service: &MetricsService, 
    session_contexts: &[sm::SessionContextData], 
    eight_contexts: &[sm::EightContextData], 
    asset_id: &str
) -> Result<Vec<mm::SessionToBarOutcomeML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("Session to Bar Outcome", asset_id, || async {
        service.calculate_session_to_bar_ml(session_contexts, eight_contexts, &intervals).await
    }).await
}

pub async fn session_to_bar_lift_rates(
    service: &MetricsService, 
    outcomes: Vec<mm::SessionToBarOutcomeML>,
    bar_base_rates: Vec<mm::BarBaseRateML>,
    asset_id: &str
) -> Result<Vec<mm::SessionToBarLiftML>> {
    run_calculation("Session to Bar Lift", asset_id, || async {
        service.calculate_session_to_bar_lift_ml(outcomes, bar_base_rates).await
    }).await
}

pub async fn daily_3rd_order_outcome_rates(
    service: &MetricsService, 
    daily_contexts: &[sm::DailyContextData], // Pass the raw daily slice
    asset_id: &str
) -> Result<Vec<mm::DailyOutcome3rdOrderML>> {
    let intervals = service.get_lookback_intervals();
    
    run_calculation("Daily 3rd Order Outcome", asset_id, || async {
        service.calculate_daily_3rd_order_ml(daily_contexts, &intervals).await
    }).await
}

pub async fn daily_3rd_order_lift_rates(
    service: &MetricsService, 
    outcomes: Vec<mm::DailyOutcome3rdOrderML>,
    daily_base_rates: Vec<mm::DailyBaseRateML>,
    asset_id: &str
) -> Result<Vec<mm::DailyOutcome3rdOrderLiftML>> {
    
    run_calculation("Daily 3rd Order Lift", asset_id, || async {
        service.calculate_daily_3rd_order_lift_ml(outcomes, daily_base_rates).await
    }).await
}

pub async fn session_continuation_rates(
    service: &MetricsService, 
    session_data: &[sm::SessionContextData], 
    asset_id: &str
) -> Result<Vec<mm::SessionContinuationML>> {
    let intervals = service.get_lookback_intervals();
    
    run_calculation("Session Continuation", asset_id, || async {
        service.calculate_session_continuation_ml(session_data, &intervals).await
    }).await
}

pub async fn bar_continuation_rates(
    service: &MetricsService, 
    bar_data: &[sm::EightContextData], 
    asset_id: &str
) -> Result<Vec<mm::BarContinuationML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("Bar Continuation", asset_id, || async {
        service.calculate_bar_continuation_ml(bar_data, &intervals).await
    }).await
}

pub async fn daily_continuation_rates(
    service: &MetricsService, 
    daily_data: &[sm::DailyContextData], 
    asset_id: &str
) -> Result<Vec<mm::DailyContinuationML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("Daily Continuation", asset_id, || async {
        service.calculate_daily_continuation_ml(daily_data, &intervals).await
    }).await
}

pub async fn session_extreme_base_rates(
    service: &MetricsService, 
    daily_raw: &[sm::DailyContextData], 
    asset_id: &str
) -> Result<Vec<mm::SessionExtremeBaseRateML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("Session Extreme Base Rate", asset_id, || async {
        service.calculate_session_extreme_base_rates_ml(daily_raw, &intervals).await
    }).await
}

pub async fn session_extreme_outcome_rates(
    service: &MetricsService, 
    session_data: &[sm::SessionContextData], 
    daily_extremes: &[sm::DailyContextData],
    asset_id: &str
) -> Result<Vec<mm::SessionExtremeOutcomeML>> {
    let intervals = service.get_lookback_intervals();
    run_calculation("Session Extreme Outcome", asset_id, || async {
        service.calculate_session_extreme_outcome_ml(session_data, daily_extremes, &intervals).await
    }).await
}

pub async fn session_extreme_lift_rates(
    service: &MetricsService, 
    outcomes: Vec<mm::SessionExtremeOutcomeML>,
    base_rates: Vec<mm::SessionExtremeBaseRateML>,
    asset_id: &str
) -> Result<Vec<mm::SessionExtremeLiftML>> {
    run_calculation("Session Extreme Lift", asset_id, || async {
        service.calculate_session_extreme_lift_ml(outcomes, base_rates).await
    }).await
}

pub async fn bar_extreme_suite(
    service: &MetricsService,
    bar_data: &[sm::EightContextData],
    daily_raw: &[sm::DailyContextData],
    asset_id: &str
) -> Result<(Vec<mm::BarExtremeBaseRateML>, Vec<mm::BarExtremeOutcomeML>, Vec<mm::BarExtremeLiftML>)> {
    
    // 1. Calculate Base Rates (Global stats)
    let base = run_calculation("Bar Extreme Base Rate", asset_id, || async {
        service.calculate_bar_extreme_base_rates_ml(daily_raw, &service.get_lookback_intervals()).await
    }).await?;

    // 2. Calculate Outcome Rates (Pattern-specific stats)
    let outcome = run_calculation("Bar Extreme Outcome", asset_id, || async {
        service.calculate_bar_extreme_outcome_ml(bar_data, daily_raw, &service.get_lookback_intervals()).await
    }).await?;

    // 3. Calculate Lift (Statistical edge)
    let lift = run_calculation("Bar Extreme Lift", asset_id, || async {
        service.calculate_bar_extreme_lift_ml(outcome.clone(), base.clone()).await
    }).await?;

    Ok((base, outcome, lift))
}