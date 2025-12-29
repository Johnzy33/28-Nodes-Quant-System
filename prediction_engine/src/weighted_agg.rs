
// // Necessary imports for the aggregator
// use std::collections::HashMap;
// use std::hash::Hash;
// use crate::{MetricsService, metrics_context::MetricsContext};
// use shared_models::models::{LookbackInterval, RawCount}; 

// // The central aggregation function
// pub fn aggregate_weighted_counts<T>(
//     data: &[T],
//     intervals: &[LookbackInterval],
//     min_attempts: f64,
// ) -> HashMap<
//     (String, T::PatternKey, T::OutcomeKey), 
//     (f64, f64) // (reliable_success_count, reliable_total_attempts)
// >
// where
//     T: MetricsContext + Clone,
//     T::PatternKey: Eq + Hash + Clone, 
//     T::OutcomeKey: Eq + Hash + Clone,
// {
//     // Key: (Asset, Pattern, Outcome, Lookback Period) -> RawCount
//     let mut raw_counts: HashMap<(String, T::PatternKey, T::OutcomeKey, String), RawCount> = HashMap::new();
    
//     // Key: (Asset, Pattern, Lookback Period) -> f64 (Total Attempts)
//     let mut raw_totals: HashMap<(String, T::PatternKey, String), f64> = HashMap::new();

//     // --- Step 1: Raw Counting (Equivalent to SQL RawCounts & RawTotals CTEs) ---
//     for item in data.iter() {
//         // Skip records where the pattern or outcome is missing
//         if let (Some(pattern_key), Some(outcome_key)) = (item.get_pattern_key(), item.get_outcome_key()) {

//             let asset_id = item.get_asset_id();
            
//             for interval in intervals.iter() {
//                 if item.get_date() >= interval.start_date {
                        
//                     // Numerator Count: Key = (Asset, Pattern, Outcome, Period)
//                     let success_key = (asset_id.clone(), pattern_key.clone(), outcome_key.clone(), interval.name.to_string());
//                     raw_counts.entry(success_key)
//                         .or_insert(RawCount { event_count: 0.0, total_attempts: 0.0 })
//                         .event_count += 1.0;
//                     // Denominator Count: Key = (Asset, Pattern, Period)
//                     let total_key = (asset_id.clone(), pattern_key.clone(), interval.name.to_string());
//                     *raw_totals.entry(total_key).or_insert(0.0) += 1.0;
//                 }
//             }
//         }
//     }

//     // --- Step 2: Apply Weights and Aggregate (Equivalent to SQL WeightedAggregates CTE) ---
//     // Final Map Key: (Asset, Pattern, Outcome) -> (Weighted Success, Weighted Total)
//     let mut weighted_results: HashMap<
//         (String, T::PatternKey, T::OutcomeKey), 
//         (f64, f64)
//     > = HashMap::new();

//     for ((asset_id, pattern_key, outcome_key, period), raw_count) in raw_counts {
//         if let Some(interval) = intervals.iter().find(|i| i.name == period.as_str()) {
            
//             // Get the total attempts for this pattern/period combination
//             let total_key = (asset_id.clone(), pattern_key.clone(), period.clone());
//             let raw_total_attempts = *raw_totals.get(&total_key).unwrap_or(&0.0);
            
//             // Apply weighting
//             let weighted_success = raw_count.event_count * interval.weight;
//             let weighted_attempts = raw_total_attempts * interval.weight;

//             // Aggregate results across all lookback periods (6M, 1Y, ALL)
//             let final_key = (asset_id, pattern_key, outcome_key);
//             let (success_agg, total_agg) = weighted_results.entry(final_key).or_insert((0.0, 0.0));

//             *success_agg += weighted_success;
//             *total_agg += weighted_attempts;
//         }
//     }

//     // --- Step 3: Filtering (WHERE reliable_total_count > N) ---
//     weighted_results.retain(|_, (_, total)| *total >= min_attempts);

//     weighted_results
// }

// impl MetricsService {
    

//     pub fn aggregate_ml_features<T>(
//     &self,
//     data: &[T],
//     intervals: &[LookbackInterval],
//     ) -> (
//         HashMap<(String, T::PatternKey, T::OutcomeKey), HashMap<String, f64>>, // Counts (Numerator)
//         HashMap<(String, T::PatternKey), HashMap<String, f64>>                // Totals (Denominator)
//     )
//     where
//         T: MetricsContext + Clone,
//         T::PatternKey: Eq + std::hash::Hash + Clone,
//         T::OutcomeKey: Eq + std::hash::Hash + Clone,
//     {
//         let mut counts = HashMap::new();
//         let mut totals = HashMap::new();

//         for item in data.iter() {
//             if let (Some(pattern), Some(outcome)) = (item.get_pattern_key(), item.get_outcome_key()) {
//                 let asset = item.get_asset_id();
//                 let date = item.get_date();

//                 for interval in intervals.iter() {
//                     if date >= interval.start_date {
//                         // 1. Numerator: (Asset, Pattern, Outcome) -> Count per Interval
//                         let count_key = (asset.clone(), pattern.clone(), outcome.clone());
//                         let interval_map = counts.entry(count_key).or_insert_with(HashMap::new);
//                         *interval_map.entry(interval.name.to_string()).or_insert(0.0) += 1.0;

//                         // 2. Denominator: (Asset, Pattern) -> Total per Interval
//                         let total_key = (asset.clone(), pattern.clone());
//                         let total_map = totals.entry(total_key).or_insert_with(HashMap::new);
//                         *total_map.entry(interval.name.to_string()).or_insert(0.0) += 1.0;
//                     }
//                 }
//             }
//         }
//         (counts, totals)
//     }

//     /// Generic helper to extract continuation probabilities (OutcomeKey = true)
//     pub fn extract_continuation_results<K, T, F>(
//         &self,
//         counts: HashMap<(String, K, bool), HashMap<String, f64>>,
//         totals: HashMap<(String, K), HashMap<String, f64>>,
//         mut map_fn: F,
//     ) -> Vec<T>
//     where
//         K: Eq + Hash + Clone,
//         F: FnMut(String, K, (f64, f64, f64), (f64, f64, f64), (f64, f64, f64)) -> T,
//     {
//         counts
//             .into_iter()
//             // We only care about cases where Continuation (OutcomeKey) was TRUE
//             .filter(|((_, _, is_cont), _)| *is_cont) 
//             .map(|(key, interval_counts)| {
//                 let (asset_id, pattern_key, _) = key;
//                 let total_key = (asset_id.clone(), pattern_key.clone());
//                 let interval_totals = totals.get(&total_key);

//                 let get_stats = |name: &str| {
//                     let count = *interval_counts.get(name).unwrap_or(&0.0);
//                     let total = interval_totals.and_then(|t| t.get(name)).cloned().unwrap_or(0.0);
//                     let prob = if total > 0.0 { count / total } else { 0.0 };
//                     (prob, count, total)
//                 };

//                 map_fn(asset_id, pattern_key, get_stats("6M"), get_stats("1Y"), get_stats("ALL"))
//             })
//             .collect()
//     }

// }