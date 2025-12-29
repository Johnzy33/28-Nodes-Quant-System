
use sqlx::PgPool;
use std::collections::HashMap;
use std::hash::Hash;
use chrono::{Utc, NaiveDate, Duration};
use shared_models as sm;
use crate::metrics_context as ctx;

pub struct MetricsService {
    pub pool: PgPool,
}

impl MetricsService {
    /// Constructs the MetricsService using the pre-initialized connection pool.
    pub fn new(pool: PgPool) -> Self {
        MetricsService { pool }
    }   
    
    // pub fn aggregate_ml_features<T>(
    // &self,
    // data: &[T],
    // intervals: &[sm::LookbackInterval],
    // ) -> (
    //     HashMap<(String, T::PatternKey, T::OutcomeKey), HashMap<String, f64>>, // Counts (Numerator)
    //     HashMap<(String, T::PatternKey), HashMap<String, f64>>                // Totals (Denominator)
    // )
    // where
    //     T: ctx::MetricsContext + Clone,
    //     T::PatternKey: Eq + std::hash::Hash + Clone,
    //     T::OutcomeKey: Eq + std::hash::Hash + Clone + Copy,
    // {
    //     let mut counts = HashMap::new();
    //     let mut totals = HashMap::new();

    //     for item in data.iter() {
    //         if let (Some(pattern), Some(outcome)) = (item.get_pattern_key(), item.get_outcome_key()) {
    //             let asset = item.get_asset_id();
    //             let date = item.get_date();

    //             for interval in intervals.iter() {
    //                 if date >= interval.start_date {
    //                     // 1. Numerator: (Asset, Pattern, Outcome) -> Count per Interval
    //                     let count_key = (asset.clone(), pattern.clone(), outcome.clone());
    //                     let interval_map = counts.entry(count_key).or_insert_with(HashMap::new);
    //                     *interval_map.entry(interval.name.to_string()).or_insert(0.0) += 1.0;

    //                     // 2. Denominator: (Asset, Pattern) -> Total per Interval
    //                     let total_key = (asset.clone(), pattern.clone());
    //                     let total_map = totals.entry(total_key).or_insert_with(HashMap::new);
    //                     *total_map.entry(interval.name.to_string()).or_insert(0.0) += 1.0;
    //                 }
    //             }
    //         }
    //     }
    //     (counts, totals)
    // }

    pub fn aggregate_ml_features<T>(
        &self,
        data: &[T],
        intervals: &[sm::LookbackInterval],
    ) -> (
        // Use fully qualified syntax for associated types in the return signature
        HashMap<(String, <T as ctx::MetricsContext>::PatternKey, <T as ctx::MetricsContext>::OutcomeKey), HashMap<String, f64>>, 
        HashMap<(String, <T as ctx::MetricsContext>::PatternKey), HashMap<String, f64>>                
    )
    where
        T: ctx::MetricsContext, 
    {
        let mut counts = HashMap::new();
        let mut totals = HashMap::new();

        for item in data {
            if let (Some(pattern), Some(outcome)) = (item.get_pattern_key(), item.get_outcome_key()) {
                let asset = item.get_asset_id();
                let date = item.get_date();

                for interval in intervals {
                    if date >= interval.start_date {
                        let interval_name = interval.name.to_string();

                        // 1. Numerator
                        let count_key = (asset.clone(), pattern.clone(), outcome.clone());
                        counts.entry(count_key)
                            .or_insert_with(HashMap::new)
                            .entry(interval_name.clone())
                            .and_modify(|c| *c += 1.0)
                            .or_insert(1.0);

                        // 2. Denominator
                        let total_key = (asset.clone(), pattern.clone());
                        totals.entry(total_key)
                            .or_insert_with(HashMap::new)
                            .entry(interval_name)
                            .and_modify(|t| *t += 1.0)
                            .or_insert(1.0);
                    }
                }
            }
        }
        (counts, totals)
    }

    pub fn get_lookback_intervals(&self) -> Vec<sm::LookbackInterval> {

        
        let today = Utc::now().date_naive();             
        let six_months_ago = today - Duration::days(180); 
        let one_year_ago = today - Duration::days(365);
        let all_history_start = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();

        vec![
            sm::LookbackInterval {
                name: "6M",
                weight: 0.60, 
                start_date: six_months_ago, 
            },
            sm::LookbackInterval {
                name: "1Y",
                weight: 0.30,
                start_date: one_year_ago, 
            },
            sm::LookbackInterval {
                name: "ALL",
                weight: 0.10,
                start_date: all_history_start, 
            },
        ]
    }

    /// Generic helper to extract continuation probabilities (OutcomeKey = true)
    pub fn extract_continuation_results<K, T, F>(
        &self,
        counts: HashMap<(String, K, bool), HashMap<String, f64>>,
        totals: HashMap<(String, K), HashMap<String, f64>>,
        mut map_fn: F,
    ) -> Vec<T>
    where
        K: Eq + Hash + Clone,
        F: FnMut(String, K, (f64, f64, f64), (f64, f64, f64), (f64, f64, f64)) -> T,
    {
        counts
            .into_iter()
            // We only care about cases where Continuation (OutcomeKey) was TRUE
            .filter(|((_, _, is_cont), _)| *is_cont) 
            .map(|(key, interval_counts)| {
                let (asset_id, pattern_key, _) = key;
                let total_key = (asset_id.clone(), pattern_key.clone());
                let interval_totals = totals.get(&total_key);

                let get_stats = |name: &str| {
                    let count = *interval_counts.get(name).unwrap_or(&0.0);
                    let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
                    let prob = if total > 0.0 { count / total } else { 0.0 };
                    (prob, count, total)
                };

                map_fn(asset_id, pattern_key, get_stats("6M"), get_stats("1Y"), get_stats("ALL"))
            })
            .collect()
    }
}


