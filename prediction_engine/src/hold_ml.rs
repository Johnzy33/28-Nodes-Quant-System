
use linfa::prelude::*;
use linfa_logistic::LogisticRegression;
use ndarray::{Array, Array2, Array1};
use std::error::Error;
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

use shared_models::models::MultiTimeframeFeatureVector;

/// The number of features in the MultiTimeframeFeatureVector that will be used for training.
/// This constant must match the number of numeric fields extracted in the feature extraction logic.
const NUM_FEATURES: usize = 19; 

/// The structure to hold the trained Logistic Regression model and its state.
#[derive(Debug, Serialize, Deserialize)]
pub struct DailyAnchorModel {
    /// The trained model instance.
    model: LogisticRegression<f64, i32>,
    /// The mapping from outcome string (e.g., "Bullish") to its integer label (e.g., 0).
    target_map: HashMap<String, i32>,
}

impl DailyAnchorModel {

    /// Converts a collection of feature vectors into an ndarray Dataset suitable for Linfa training.
    /// It separates features (X) from the target label (Y) and maps target strings to integers.
    fn create_dataset(
        data: &[MultiTimeframeFeatureVector]
    ) -> Result<(DatasetBase<Array2<f64>, Array1<i32>>, HashMap<String, i32>), Box<dyn Error>> {

        if data.is_empty() {
            return Err("Input data is empty.".into());
        }

        let num_samples = data.len();
        let mut features_data = Vec::with_capacity(num_samples * NUM_FEATURES);
        let mut target_labels = Vec::with_capacity(num_samples);
        let mut target_map = HashMap::new();
        let mut next_label = 0;

        for record in data {
            // --- 1. Extract Features (Must match the order in daily_anchor_pipeline.rs) ---
            features_data.push(record.dci_long_max_score);
            features_data.push(record.dci_short_max_score);
            features_data.push(record.pdcs_anchor_max_score);
            features_data.push(record.p_dci_conditional_max);
            features_data.push(record.p_day_type_base_for_max);
            features_data.push(record.p_daily_trend_rate_raw);
            
            features_data.push(record.fci_long_max_score);
            features_data.push(record.fci_short_max_score);
            features_data.push(record.tcs_anchor_max_score);
            features_data.push(record.p_cs_long_cond_max);
            features_data.push(record.p_cs_short_cond_max);
            features_data.push(record.p_tcs_continuation_raw);
            features_data.push(record.p_high_session_cond_max);
            features_data.push(record.p_low_session_cond_max);
            
            features_data.push(record.wci_long_max_score);
            features_data.push(record.wci_short_max_score);
            features_data.push(record.wocs_d_anchor_max);
            features_data.push(record.wcs_continuation_max);
            features_data.push(record.wms_seasonality_max);

            // --- 2. Encode Target ---
            let outcome = &record.target_day_outcome;
            let label = *target_map.entry(outcome.clone()).or_insert_with(|| {
                let current_label = next_label;
                next_label += 1;
                current_label
            });
            target_labels.push(label);
        }

        let features_array = Array2::from_shape_vec((num_samples, NUM_FEATURES), features_data)?;
        let targets_array = Array1::from_vec(target_labels);

        let dataset = DatasetBase::new(features_array, targets_array);

        Ok((dataset, target_map))
    }


    /// Trains the Logistic Regression model using the provided feature vectors.
    pub fn train(data: &[MultiTimeframeFeatureVector]) -> Result<Self, Box<dyn Error>> {
        
        let (dataset, target_map) = Self::create_dataset(data)?;

        // Linfa Logistic Regression - common settings for multi-class classification
        let model = LogisticRegression::default()
            .max_iterations(100)
            .l2_penalty(0.5) // Small L2 penalty for regularization
            .fit(&dataset)?;

        println!("Daily Anchor Model (Model 1) trained successfully. Classes: {}", target_map.len());
        
        Ok(DailyAnchorModel { model, target_map })
    }

    /// Predicts the `target_day_outcome` for a single feature vector.
    pub fn predict(&self, feature_vector: &MultiTimeframeFeatureVector) -> Result<String, Box<dyn Error>> {
        
        // Extract features into an Array2<f64> suitable for prediction (1 row, NUM_FEATURES columns)
        let features_data = vec![
            feature_vector.dci_long_max_score, feature_vector.dci_short_max_score, feature_vector.pdcs_anchor_max_score,
            feature_vector.p_dci_conditional_max, feature_vector.p_day_type_base_for_max, feature_vector.p_daily_trend_rate_raw,
            feature_vector.fci_long_max_score, feature_vector.fci_short_max_score, feature_vector.tcs_anchor_max_score,
            feature_vector.p_cs_long_cond_max, feature_vector.p_cs_short_cond_max, feature_vector.p_tcs_continuation_raw,
            feature_vector.p_high_session_cond_max, feature_vector.p_low_session_cond_max,
            feature_vector.wci_long_max_score, feature_vector.wci_short_max_score, feature_vector.wocs_d_anchor_max,
            feature_vector.wcs_continuation_max, feature_vector.wms_seasonality_max,
        ];
        
        let features_array = Array2::from_shape_vec((1, NUM_FEATURES), features_data)?;
        
        let prediction_result = self.model.predict(&features_array);
        
        if let Some(predicted_label) = prediction_result.get(0) {
            // Find the original outcome string from the stored map
            let outcome = self.target_map.iter()
                .find(|(_, &label)| label == *predicted_label)
                .map(|(outcome, _)| outcome.clone())
                .ok_or("Prediction label not found in target map.")?;

            Ok(outcome)
        } else {
            Err("Prediction failed to produce a result.".into())
        }
    }

    /// Saves the trained model state and target map to a file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let serialized = serde_json::to_string(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Loads the trained model state and target map from a file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let serialized = fs::read_to_string(path)?;
        let model = serde_json::from_str(&serialized)?;
        Ok(model)
    }
}

//------------------ for Session

use linfa::prelude::*;
use linfa_logistic::LogisticRegression;
use ndarray::{Array, Array2, Array1};
use std::error::Error;
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use shared_models::models::SessionFeatureVector;

/// The number of features in the SessionFeatureVector used for training.
const NUM_FEATURES: usize = 7; 

/// The structure to hold the trained Logistic Regression model and its state for Model 2.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionConfidenceModel {
    /// The trained model instance.
    model: LogisticRegression<f64, i32>,
    /// The mapping from outcome string (e.g., "Long", "Short") to its integer label (e.g., 0, 1).
    target_map: HashMap<String, i32>,
    /// The mapping from the categorical ML Daily Prediction (Model 1 output) to numerical features.
    daily_prediction_map: HashMap<String, i32>,
}

impl SessionConfidenceModel {

    /// Converts a collection of feature vectors into an ndarray Dataset suitable for Linfa training.
    /// This includes mapping all categorical string features (ML Anchor prediction) to integers.
    fn create_dataset(
        data: &[SessionFeatureVector]
    ) -> Result<(DatasetBase<Array2<f64>, Array1<i32>>, HashMap<String, i32>, HashMap<String, i32>), Box<dyn Error>> {

        if data.is_empty() {
            return Err("Input data is empty.".into());
        }

        let num_samples = data.len();
        let mut features_data = Vec::with_capacity(num_samples * NUM_FEATURES);
        let mut target_labels = Vec::with_capacity(num_samples);
        
        let mut target_map = HashMap::new();
        let mut daily_prediction_map = HashMap::new();
        let mut next_target_label = 0;
        let mut next_daily_pred_label = 0;

        for record in data {
            
            // --- 1. Encode Categorical Features (ML Daily Prediction) ---
            let daily_pred_label = *daily_prediction_map.entry(record.ml_daily_prediction.clone()).or_insert_with(|| {
                let current_label = next_daily_pred_label;
                next_daily_pred_label += 1;
                current_label
            });
            
            // --- 2. Extract Features (7 features) ---
            // Note: For simplicity in this base model, we use the integer label of the categorical feature
            // as one of the inputs. In production, this would be one-hot encoded (but Linfa handles a subset 
            // of features for Logistic Regression).
            features_data.push(record.p_cs_long_cond);
            features_data.push(record.p_cs_short_cond);
            features_data.push(record.tcs_score);
            features_data.push(record.p_tcs_continuation);
            
            // Model 1 Prediction (Categorical feature is now integer)
            features_data.push(daily_pred_label as f64);
            
            // Model 1 Confidence Score
            features_data.push(record.ml_daily_score);

            // Placeholder for the single numeric feature derived from PS1/PS2 names/biases 
            // (These should be one-hot encoded, but for now we'll use a constant and adjust NUM_FEATURES later)
            features_data.push(0.0); // Feature 7 placeholder (e.g., interaction term or one-hot index)


            // --- 3. Encode Target ---
            let outcome = &record.target_cs_bias;
            let label = *target_map.entry(outcome.clone()).or_insert_with(|| {
                let current_label = next_target_label;
                next_target_label += 1;
                current_label
            });
            target_labels.push(label);
        }

        let features_array = Array2::from_shape_vec((num_samples, NUM_FEATURES), features_data)?;
        let targets_array = Array1::from_vec(target_labels);

        let dataset = DatasetBase::new(features_array, targets_array);

        Ok((dataset, target_map, daily_prediction_map))
    }


    /// Trains the Logistic Regression model using the provided session feature vectors.
    pub fn train(data: &[SessionFeatureVector]) -> Result<Self, Box<dyn Error>> {
        
        let (dataset, target_map, daily_prediction_map) = Self::create_dataset(data)?;

        // Linfa Logistic Regression - settings for this binary/multi-class classification
        let model = LogisticRegression::default()
            .max_iterations(100)
            .l2_penalty(0.5) 
            .fit(&dataset)?;

        println!("Session Confidence Model (Model 2) trained successfully. Target Classes: {}, Prediction Labels: {}", 
            target_map.len(), daily_prediction_map.len());
        
        Ok(SessionConfidenceModel { model, target_map, daily_prediction_map })
    }

    /// Predicts the `target_cs_bias` for a single feature vector.
    pub fn predict(&self, feature_vector: &SessionFeatureVector) -> Result<String, Box<dyn Error>> {
        
        // 1. Convert Model 1 categorical prediction to the integer label used in training
        let daily_pred_label = *self.daily_prediction_map
            .get(&feature_vector.ml_daily_prediction)
            .ok_or("ML Daily Prediction label not found in model's map.")?;

        // 2. Extract features into an Array2<f64>
        let features_data = vec![
            feature_vector.p_cs_long_cond,
            feature_vector.p_cs_short_cond,
            feature_vector.tcs_score,
            feature_vector.p_tcs_continuation,
            daily_pred_label as f64, // Feature 5
            feature_vector.ml_daily_score, // Feature 6
            0.0, // Feature 7 placeholder
        ];
        
        let features_array = Array2::from_shape_vec((1, NUM_FEATURES), features_data)?;
        
        let prediction_result = self.model.predict(&features_array);
        
        if let Some(predicted_label) = prediction_result.get(0) {
            // Find the original outcome string from the stored map
            let outcome = self.target_map.iter()
                .find(|(_, &label)| label == *predicted_label)
                .map(|(outcome, _)| outcome.clone())
                .ok_or("Prediction label not found in target map.")?;

            Ok(outcome)
        } else {
            Err("Prediction failed to produce a result.".into())
        }
    }

    /// Saves the trained model state and target map to a file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let serialized = serde_json::to_string(self)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Loads the trained model state and target map from a file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let serialized = fs::read_to_string(path)?;
        let model = serde_json::from_str(&serialized)?;
        Ok(model)
    }
}


// ---------------------

use std::collections::HashMap;
use chrono::NaiveDate;

use shared_models::models::{
    DailyConfidenceScore, Tcs2ndOrder, TcsContinuationScore,
    HighLowSession2ndOrder, WeeklyContinuationScore, WeeklyMonthlySeasonality, WocsDailyScore,
    MultiTimeframeFeatureVector, DailyOutcome, WeeklyBaseRate, DayTypeBaseRate,
};

// --- Helper Functions from PL/pgSQL Logic (DCI & FCI) ---

/// Determines the bias group (Long or Short) for a given Day/Week Outcome.
fn get_bias_group(day_type: &str) -> Option<&'static str> {
    match day_type {
        "Bullish" | "Bullish_Reversal" | "Failed_Bullish" | "Consolidation-Up" => Some("Long"),
        "Bearish" | "Bearish_Reversal" | "Failed_Bearish" | "Consolidation-Down" => Some("Short"),
        _ => None,
    }
}

// FCI: Calculates the robust P_Continuation lift multiplier (LIFT 1)
fn calculate_fci_continuation_lift(p_continuation: f64) -> f64 {
    if p_continuation >= 80.0 {
        0.30 // 30% lift
    } else if p_continuation >= 65.0 {
        0.15 // 15% lift
    } else {
        0.00
    }
}

/// FCI: Implements the complex FCI calculation logic derived from the PL/pgSQL function.
/// FCI = (((PDCS_Base * P_Cond / 100) * TCS_Multiplier) * (1 + Continuation_Lift)) * 100
/// NOTE: The PL/pgSQL for FCI combined DCI and TCS. We model this closely.
fn calculate_fci_score(
    pdcs_score_anchor: f64, // P_cond / P_base, which serves as a factor
    p_day_type_base: f64,   // P_Day_Type_Base (P(D0))
    tcs_multiplier: f64,    // TCS score (P_cond / P_base, factor)
    p_continuation_raw: f64,
) -> f64 {
    
    let base_prob = p_day_type_base / 100.0;
    let continuation_lift = calculate_fci_continuation_lift(p_continuation_raw);
    
    let fci_raw = (pdcs_score_anchor * base_prob) * tcs_multiplier * (1.0 + continuation_lift);
    
    (fci_raw * 100.0 * 100.0).round() / 100.0
}

// WCI: Calculates the Weekly Continuation Score (WCS) lift multiplier (LIFT 2)
fn calculate_wcs_lift(wcs_score: f64) -> f64 {
    if wcs_score >= 80.0 {
        0.30
    } else if wcs_score >= 65.0 {
        0.15
    } else {
        0.00
    }
}

// WCI: Calculates the Weekly Monthly Seasonality (WMS) lift multiplier (LIFT 3)
fn calculate_wms_lift(wms_score: f64) -> f64 {
    if wms_score >= 75.0 {
        0.20
    } else if wms_score >= 60.0 {
        0.10
    } else {
        0.00
    }
}

/// WCI: Implements the WCI calculation logic from the PL/pgSQL function.
/// WCI = (((WOCS-D * WBR / 100) * (1 + WCS_Lift) * (1 + WMS_Lift)) * 100)
fn calculate_wci_score(
    wocs_d_score: f64,
    weekly_base_rate: f64, // P(W0)
    wcs_score: f64,
    wms_score: f64,
) -> f64 {
    let base_prob = weekly_base_rate / 100.0;
    let wcs_lift = calculate_wcs_lift(wcs_score);
    let wms_lift = calculate_wms_lift(wms_score);

    let wci_raw = (wocs_d_score * base_prob) * (1.0 + wcs_lift) * (1.0 + wms_lift);

    (wci_raw * 100.0 * 100.0).round() / 100.0
}

// --- Main Feature Aggregation Pipeline ---

/// Generates the final MultiTimeframeFeatureVector by joining and aggregating all metric tables.
/// This vector is the input (X) for the Static Daily Anchor (Model 1).
#[allow(clippy::too_many_arguments)]
pub fn generate_multi_timeframe_features(
    daily_outcomes: &[DailyOutcome],
    pdcs_scores: &[DailyConfidenceScore],
    tcs_scores: &[Tcs2ndOrder],
    tcs_cont_scores: &[TcsContinuationScore],
    high_low_scores: &[HighLowSession2ndOrder],
    wocs_scores: &[WocsDailyScore],
    wcs_scores: &[WeeklyContinuationScore],
    wms_scores: &[WeeklyMonthlySeasonality],
    // Required to calculate the WCI Score fully
    weekly_base_rates: &[WeeklyBaseRate], 
    day_type_base_rates: &[DayTypeBaseRate], 
) -> Vec<MultiTimeframeFeatureVector> {

    // --- 1. Prepare Data for Date/Asset Lookup ---

    // PDCS Map: Key (Asset, DOW) -> Vec<DailyConfidenceScore>
    let pdcs_map: HashMap<(String, String), Vec<&DailyConfidenceScore>> = pdcs_scores.iter()
        .map(|s| ((s.asset_id.clone(), s.pd1_dow.clone()), s))
        .fold(HashMap::new(), |mut acc, (key, score)| {
            acc.entry(key).or_insert_with(Vec::new).push(score);
            acc
        });
    
    // Day Type Base Rate Map: Key (Asset, Outcome) -> P_base
    let day_base_map: HashMap<(String, String), f64> = day_type_base_rates.iter()
        .map(|b| ((b.asset_id.clone(), b.daily_outcome.clone()), b.p_day_type_base))
        .collect();

    // High/Low Map: Key (Asset) -> Vec<HighLowSession2ndOrder>
    let high_low_map: HashMap<String, Vec<&HighLowSession2ndOrder>> = high_low_scores.iter()
        .fold(HashMap::new(), |mut acc, score| {
            acc.entry(score.asset_id.clone()).or_insert_with(Vec::new).push(score);
            acc
        });
        
    // TCS Map: Key (Asset) -> Vec<Tcs2ndOrder>
    let tcs_map: HashMap<String, Vec<&Tcs2ndOrder>> = tcs_scores.iter()
        .fold(HashMap::new(), |mut acc, score| {
            acc.entry(score.asset_id.clone()).or_insert_with(Vec::new).push(score);
            acc
        });

    // TCS Continuation Map: Key (Asset, PS2 Bias, PS1 Bias) -> P_Continuation
    let tcs_cont_map: HashMap<(String, String, String), f64> = tcs_cont_scores.iter()
        .map(|s| ((s.asset_id.clone(), s.ps2_bias.clone(), s.ps1_bias.clone()), s.p_continuation))
        .collect();

    // Weekly Base Rate Map: Key (Asset, Outcome) -> Weekly_Base_Rate
    let weekly_base_map: HashMap<(String, String), f64> = weekly_base_rates.iter()
        .map(|b| ((b.asset_id.clone(), b.week0_outcome.clone()), b.weekly_base_rate))
        .collect();


    // --- 2. Iterate over all Daily Outcomes (Targets) to build features for each date ---
    let mut feature_vectors: Vec<MultiTimeframeFeatureVector> = Vec::new();

    for outcome in daily_outcomes {
        let asset_id = &outcome.asset_id;
        let trading_date = outcome.trading_date;
        let day_of_week_str = trading_date.format("%a").to_string(); 

        // --- 2A. Daily Confidence Index (DCI) Aggregation ---
        let mut dci_long_max_score = 0.0;
        let mut dci_short_max_score = 0.0;
        let mut pdcs_anchor_max_score = 0.0;
        let mut p_dci_conditional_max = 0.0;
        let mut p_day_type_base_for_max = 0.0;
        let mut p_daily_trend_rate_raw = 0.0; // Placeholder until DailyTrendRate table is available

        // DCI calculation uses P(D0 | PD2, PD1, DOW) * (1 + Trend Lift)
        if let Some(scores) = pdcs_map.get(&(asset_id.clone(), day_of_week_str.clone())) {
            for score in scores.iter() {
                // In the original PL/pgSQL, DCI = P_cond * (1 + Lift).
                let dci_score = score.p_day_outcome_conditional * (1.0 + 
                    // Trend rate raw placeholder is 0.0 for now, so lift is 0.0
                    if p_daily_trend_rate_raw >= 80.0 { 0.30 } else if p_daily_trend_rate_raw >= 65.0 { 0.15 } else { 0.00 }
                );

                if score.pdcs_score > pdcs_anchor_max_score {
                    pdcs_anchor_max_score = score.pdcs_score;
                    p_dci_conditional_max = score.p_day_outcome_conditional;
                    p_day_type_base_for_max = day_base_map.get(&(asset_id.clone(), score.day0_outcome.clone())).copied().unwrap_or(0.0);
                }
                
                let bias_group = get_bias_group(&score.day0_outcome);
                let is_vetoed = score.pdcs_score < 0.75; 

                if !is_vetoed {
                    if let Some("Long") = bias_group {
                        dci_long_max_score = dci_long_max_score.max(dci_score);
                    } else if let Some("Short") = bias_group {
                        dci_short_max_score = dci_short_max_score.max(dci_score);
                    }
                }
            }
        }
        
        // --- 2B. Flow Confidence Index (FCI) Aggregation ---
        let mut fci_long_max_score = 0.0;
        let mut fci_short_max_score = 0.0;
        let mut tcs_anchor_max_score = 0.0;
        let mut p_cs_long_cond_max = 0.0;
        let mut p_cs_short_cond_max = 0.0;
        let mut p_tcs_continuation_raw = 0.0;

        if let Some(tcs_scores) = tcs_map.get(asset_id) {
            for tcs in tcs_scores.iter() {
                let key = (asset_id.clone(), tcs.ps2_bias.clone(), tcs.ps1_bias.clone());
                let p_cont = tcs_cont_map.get(&key).copied().unwrap_or(0.0);
                
                // For FCI calculation, we must use the actual P_Day_Type_Base (P(D0))
                let p_day_base_for_fci = day_base_map.get(&(asset_id.clone(), tcs.cs_bias.clone())).copied().unwrap_or(0.0);
                
                let fci_score = calculate_fci_score(
                    tcs.p_cs_base, // P_CS_Base is used as P_Cond/P_Base anchor in this context
                    p_day_base_for_fci, 
                    tcs.tcs_score, 
                    p_cont, 
                ); 
                
                let bias_group = get_bias_group(&tcs.cs_bias);
                let is_vetoed = tcs.tcs_score < 0.50; // TCS Veto Logic

                if tcs.tcs_score > tcs_anchor_max_score { tcs_anchor_max_score = tcs.tcs_score; }
                if p_cont > p_tcs_continuation_raw { p_tcs_continuation_raw = p_cont; }
                
                if !is_vetoed {
                    if let Some("Long") = bias_group {
                        if fci_score > fci_long_max_score {
                            fci_long_max_score = fci_score;
                            p_cs_long_cond_max = tcs.p_transition_conditional;
                        }
                    } else if let Some("Short") = bias_group {
                        if fci_score > fci_short_max_score {
                            fci_short_max_score = fci_score;
                            p_cs_short_cond_max = tcs.p_transition_conditional;
                        }
                    }
                }
            }
        }
        
        // --- 2C. Session Extremes Aggregation ---
        let mut p_high_session_cond_max = 0.0;
        let mut p_low_session_cond_max = 0.0;

        if let Some(scores) = high_low_map.get(asset_id) {
            for score in scores.iter() {
                if score.session_extreme == "High" {
                    p_high_session_cond_max = p_high_session_cond_max.max(score.p_extreme_session_conditional);
                } else if score.session_extreme == "Low" {
                    p_low_session_cond_max = p_low_session_cond_max.max(score.p_extreme_session_conditional);
                }
            }
        }
        
        // --- 2D. Weekly Confidence Index (WCI) Aggregation ---
        let mut wci_long_max_score = 0.0;
        let mut wci_short_max_score = 0.0;
        let mut wocs_d_anchor_max = 0.0;
        let mut wcs_continuation_max = 0.0;
        let mut wms_seasonality_max = 0.0;
        
        // WOCS-D (Weekly Outcome Conditional on Daily Score) Aggregation
        // WCI uses a complex join, we'll iterate through all WOCS scores for the asset
        // and find the maximum associated multipliers from WCS and WMS
        
        for wocs in wocs_scores.iter().filter(|w| w.asset_id == *asset_id) {
            wocs_d_anchor_max = wocs_d_anchor_max.max(wocs.wocs_d_score);
            
            // 1. Get associated WCS and WMS scores for this WOCS pattern
            let wcs = wcs_scores.iter()
                .filter(|w| w.asset_id == *asset_id && w.week0_outcome == wocs.week0_outcome)
                .map(|w| w.wcs_score)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);
            
            wcs_continuation_max = wcs_continuation_max.max(wcs);

            let wms = wms_scores.iter()
                .filter(|w| w.asset_id == *asset_id && w.week0_outcome == wocs.week0_outcome)
                .map(|w| w.wms_score)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);
            
            wms_seasonality_max = wms_seasonality_max.max(wms);

            let weekly_base_rate = weekly_base_map.get(&(asset_id.clone(), wocs.week0_outcome.clone())).copied().unwrap_or(0.0);

            // 2. Calculate the WCI score for this specific outcome/pattern
            let wci_score = calculate_wci_score(
                wocs.wocs_d_score,
                weekly_base_rate,
                wcs,
                wms,
            );

            // 3. Aggregate Max WCI scores based on the outcome group
            let bias_group = get_bias_group(&wocs.week0_outcome);
            let is_vetoed = wocs.wocs_d_score < 60.0; // WCI Veto Logic

            if !is_vetoed {
                if let Some("Long") = bias_group {
                    wci_long_max_score = wci_long_max_score.max(wci_score);
                } else if let Some("Short") = bias_group {
                    wci_short_max_score = wci_short_max_score.max(wci_score);
                }
            }
        }
        
        // --- 3. Build the Final Vector ---
        feature_vectors.push(MultiTimeframeFeatureVector {
            trading_date,
            asset_id: asset_id.clone(),

            // DCI Signals (6 features)
            dci_long_max_score,
            dci_short_max_score,
            pdcs_anchor_max_score,
            p_dci_conditional_max,
            p_day_type_base_for_max,
            p_daily_trend_rate_raw, // P_Trend_Rate_Raw needs calculation from DailyTrendRate table

            // FCI Signals (8 features)
            fci_long_max_score,
            fci_short_max_score,
            tcs_anchor_max_score,
            p_cs_long_cond_max,
            p_cs_short_cond_max,
            p_tcs_continuation_raw,
            p_high_session_cond_max,
            p_low_session_cond_max,

            // WCI Signals (5 features)
            wci_long_max_score,
            wci_short_max_score,
            wocs_d_anchor_max,
            wcs_continuation_max,
            wms_seasonality_max,

            // Targets (Daily and Weekly)
            target_day_outcome: outcome.day_outcome.clone(),
            target_week_outcome: outcome.week_outcome.clone(),
        });
    }

    feature_vectors
}

//

use std::collections::HashMap;
use chrono::NaiveDate;

// Assume this module is in scope via `use` (e.g., `shared_models::*`)
use crate::shared_models::{
    DailyAnchorFeatureVector, DailyOutcome, DailyConfidenceScore, DailyTrendRate, DayTypeBaseRate,
    WocsDailyScore, WeeklyContinuationScore, WeeklyMonthlySeasonality, HighLowSession2ndOrder,
};


/// Determines the bias group (Long or Short) for a given Day/Session/Week Outcome.
/// This is used for correctly grouping the conditional probabilities.
fn get_bias_group(outcome: &str) -> Option<&'static str> {
    match outcome {
        // Long Group (Bullish family)
        "Bullish" | "Bullish_Reversal" | "Consolidation-Up" | "Failed_Bearish" => Some("Long"),
        // Short Group (Bearish family)
        "Bearish" | "Bearish_Reversal" | "Consolidation-Down" | "Failed_Bullish" => Some("Short"),
        _ => None, // Neutral or other biases are ignored for Max Long/Short scoring
    }
}


/// Generates the final DailyAnchorFeatureVector by joining and aggregating all static metric tables.
/// This 15-feature vector is the clean, raw input (X) for the Static Daily Anchor (Model 1).
#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
pub fn generate_daily_anchor_features(
    daily_outcomes: &[DailyOutcome],
    pdcs_scores: &[DailyConfidenceScore],
    daily_trend_rates: &[DailyTrendRate],
    day_type_base_rates: &[DayTypeBaseRate],
    wocs_scores: &[WocsDailyScore],
    wcs_scores: &[WeeklyContinuationScore],
    wms_scores: &[WeeklyMonthlySeasonality],
    high_low_scores: &[HighLowSession2ndOrder],
) -> Vec<DailyAnchorFeatureVector> {

    // --- 1. Prepare Data for Date/Asset Lookup ---

    // PDCS Map: Key (Asset, DOW) -> Vec<DailyConfidenceScore>
    let pdcs_map: HashMap<(String, String), Vec<&DailyConfidenceScore>> = pdcs_scores.iter()
        .map(|s| ((s.asset_id.clone(), s.pd1_dow.clone()), s))
        .fold(HashMap::new(), |mut acc, (key, score)| {
            acc.entry(key).or_insert_with(Vec::new).push(score);
            acc
        });
    
    // Day Type Base Rate Map: Key (Asset, Outcome) -> P_base
    let day_base_map: HashMap<(String, String), f64> = day_type_base_rates.iter()
        .map(|b| ((b.asset_id.clone(), b.daily_outcome.clone()), b.p_day_type_base))
        .collect();

    // Daily Trend Rate Map: Key (Asset, Date) -> Trend Rate
    let trend_map: HashMap<(String, NaiveDate), f64> = daily_trend_rates.iter()
        // Assuming we need to map the trend rate to the specific day it applies to
        // NOTE: A proper implementation would need a Date field in DailyTrendRate
        .map(|r| ((r.asset_id.clone(), NaiveDate::from_ymd_opt(1, 1, 1).unwrap()), r.trend_rate)) // Placeholder date for now
        .collect();


    // --- 2. Aggregate Static FCI Extremes (Timeless/Pattern-Based) ---
    
    // This aggregates the HighLowSession2ndOrder table into the four MAX scores
    // Key: Asset ID
    // Value: (max_high_long, max_high_short, max_low_long, max_low_short)
    let mut hl_agg_map: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();

    for score in high_low_scores {
        let entry = hl_agg_map.entry(score.asset_id.clone()).or_insert((0.0, 0.0, 0.0, 0.0));
        let p = score.p_extreme_session_conditional;
        let bias_group = get_bias_group(&score.cs_bias);

        if let Some("Long") = bias_group {
            if score.session_extreme == "High" {
                entry.0 = f64::max(entry.0, p);
            } else if score.session_extreme == "Low" {
                entry.2 = f64::max(entry.2, p);
            }
        } else if let Some("Short") = bias_group {
            if score.session_extreme == "High" {
                entry.1 = f64::max(entry.1, p);
            } else if score.session_extreme == "Low" {
                entry.3 = f64::max(entry.3, p);
            }
        }
    }


    // --- 3. Aggregate Static WCI Scores (Timeless/Pattern-Based) ---
    
    // Group all WCI related scores by (Asset ID, Weekly Outcome)
    type WeeklyKey = (String, String);
    struct WeeklyAggMetrics {
        wocs_d: f64, // WOCS-D anchor score
        wcs: f64,    // WCS continuation score
        wms: f64,    // WMS seasonality score
    }
    
    let mut weekly_agg_map: HashMap<WeeklyKey, WeeklyAggMetrics> = HashMap::new();
    
    for wocs in wocs_scores {
        let key: WeeklyKey = (wocs.asset_id.clone(), wocs.week0_outcome.clone());
        let entry = weekly_agg_map.entry(key).or_insert(WeeklyAggMetrics { wocs_d: 0.0, wcs: 0.0, wms: 0.0 });
        entry.wocs_d = f64::max(entry.wocs_d, wocs.wocs_d_score);
    }
    for wcs in wcs_scores {
        let key: WeeklyKey = (wcs.asset_id.clone(), wcs.week0_outcome.clone());
        let entry = weekly_agg_map.entry(key).or_insert(WeeklyAggMetrics { wocs_d: 0.0, wcs: 0.0, wms: 0.0 });
        entry.wcs = f64::max(entry.wcs, wcs.wcs_score);
    }
    for wms in wms_scores {
        let key: WeeklyKey = (wms.asset_id.clone(), wms.week0_outcome.clone());
        let entry = weekly_agg_map.entry(key).or_insert(WeeklyAggMetrics { wocs_d: 0.0, wcs: 0.0, wms: 0.0 });
        entry.wms = f64::max(entry.wms, wms.wms_score);
    }

    
    // --- 4. Iterate over all Daily Outcomes (Targets) to build features for each date ---
    let mut feature_vectors: Vec<DailyAnchorFeatureVector> = Vec::new();

    for outcome in daily_outcomes {
        let asset_id = &outcome.asset_id;
        let trading_date = outcome.trading_date;
        let day_of_week_str = trading_date.format("%a").to_string(); 

        // --- 4A. DCI Aggregation ---
        let mut dci_long_max_score = 0.0;
        let mut dci_short_max_score = 0.0;
        let mut pdcs_anchor_max_score = 0.0;
        let mut p_dci_conditional_max = 0.0;
        let mut p_day_type_base_for_max = 0.0;
        
        // Lookup Daily Trend Rate
        let p_daily_trend_rate_raw = trend_map.get(&(asset_id.clone(), trading_date)).copied().unwrap_or(0.0);

        // PDCS calculation looks up the Day of Week (PD1_DOW) and previous days (PD2, PD1)
        if let Some(scores) = pdcs_map.get(&(asset_id.clone(), day_of_week_str.clone())) {
            for score in scores.iter() {
                let p_cond = score.p_day_outcome_conditional;
                let raw_pdcs_score = score.pdcs_score; // Raw Anchor

                // Find the maximum P_Conditional for all PDCS patterns
                if p_cond > p_dci_conditional_max {
                    p_dci_conditional_max = p_cond;
                    pdcs_anchor_max_score = raw_pdcs_score;
                    // Find the base rate associated with the pattern that gave the max conditional P
                    p_day_type_base_for_max = day_base_map.get(&(asset_id.clone(), score.day0_outcome.clone())).copied().unwrap_or(0.0);
                }
                
                let bias_group = get_bias_group(&score.day0_outcome);

                // Find the Max Raw Anchor Score for Long and Short groups
                if let Some("Long") = bias_group {
                    dci_long_max_score = dci_long_max_score.max(raw_pdcs_score);
                } else if let Some("Short") = bias_group {
                    dci_short_max_score = dci_short_max_score.max(raw_pdcs_score);
                }
            }
        }
        
        // --- 4B. WCI Aggregation ---
        let mut wci_long_max_score = 0.0;
        let mut wci_short_max_score = 0.0;
        let mut wocs_d_anchor_max = 0.0;
        let mut wcs_continuation_max = 0.0;
        let mut wms_seasonality_max = 0.0;
        
        // WCI calculation aggregates all weekly-related metrics (WOCS, WCS, WMS) for the asset
        for (key, metrics) in &weekly_agg_map {
            if key.0 != *asset_id { continue; } // Skip other assets
            
            wocs_d_anchor_max = wocs_d_anchor_max.max(metrics.wocs_d);
            wcs_continuation_max = wcs_continuation_max.max(metrics.wcs);
            wms_seasonality_max = wms_seasonality_max.max(metrics.wms);

            let bias_group = get_bias_group(&key.1); // key.1 is week0_outcome

            // Find the Max Raw Anchor Score (WOCS-D) for Long and Short groups
            if let Some("Long") = bias_group {
                wci_long_max_score = wci_long_max_score.max(metrics.wocs_d);
            } else if let Some("Short") = bias_group {
                wci_short_max_score = wci_short_max_score.max(metrics.wocs_d);
            }
        }

        // --- 4C. Static FCI Extremes Aggregation ---
        
        let (p_high_long, p_high_short, p_low_long, p_low_short) = 
            hl_agg_map.get(asset_id).copied().unwrap_or((0.0, 0.0, 0.0, 0.0));
        
        // --- 5. Build the Final Vector ---
        feature_vectors.push(DailyAnchorFeatureVector {
            trading_date,
            asset_id: asset_id.clone(),

            // DCI Signals
            dci_long_max_score,
            dci_short_max_score,
            pdcs_anchor_max_score,
            p_dci_conditional_max,
            p_day_type_base_for_max,
            p_daily_trend_rate_raw,

            // WCI Signals
            wci_long_max_score,
            wci_short_max_score,
            wocs_d_anchor_max,
            wcs_continuation_max,
            wms_seasonality_max,

            // Static Session Extremes
            p_high_session_cond_long_max: p_high_long,
            p_high_session_cond_short_max: p_high_short,
            p_low_session_cond_long_max: p_low_long,
            p_low_session_cond_short_max: p_low_short,

            // Targets
            target_day_outcome: outcome.day_outcome.clone(),
            target_week_outcome: outcome.week_outcome.clone(),
        });
    }

    feature_vectors
}