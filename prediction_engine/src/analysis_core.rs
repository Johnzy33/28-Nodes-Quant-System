use shared_models::tcs_models::{
    Tcs1stOrderData, Tcs2ndOrderData, TcsLookbackSummary, PredictionPoint, CrossOrderComparison, TcsFinalReport
};
use shared_models::tcs_analysis_config::TcsThresholds;
use std::collections::HashMap;

// --- Phase 2.3: Summary Generation ---

/// Generates a summary for a single TCS metric (e.g., 1st Order) across all lookback periods.
pub fn generate_single_metric_summary(
    data: Vec<Tcs1stOrderData>
) -> HashMap<String, TcsLookbackSummary> {
    // Implementation details: Grouping data by lookback period and finding the extrema...

    let mut grouped_data: HashMap<String, Vec<Tcs1stOrderData>> = HashMap::new();
    for item in data {
        grouped_data.entry(item.lookback_period.clone()).or_default().push(item);
    }
    
    let mut final_summaries: HashMap<String, TcsLookbackSummary> = HashMap::new();

    for (lookback, records) in grouped_data {
        if records.is_empty() { continue; }

        let mut strongest_edge: Option<&Tcs1stOrderData> = None;
        let mut strongest_veto: Option<&Tcs1stOrderData> = None;
        let mut most_likely: Option<&Tcs1stOrderData> = None;

        for record in records.iter() {
            if strongest_edge.is_none() || record.tcs_score > strongest_edge.unwrap().tcs_score {
                strongest_edge = Some(record);
            }
            if strongest_veto.is_none() || record.tcs_score < strongest_veto.unwrap().tcs_score {
                strongest_veto = Some(record);
            }
            if most_likely.is_none() || record.p_transition_conditional > most_likely.unwrap().p_transition_conditional {
                most_likely = Some(record);
            }
        }
        
        if let (Some(edge), Some(veto), Some(likely)) = (strongest_edge, strongest_veto, most_likely) {
            let summary = TcsLookbackSummary {
                lookback_period: lookback.clone(),
                strongest_edge: PredictionPoint {
                    bias: edge.predicted_cs_bias.clone(),
                    score: edge.tcs_score,
                    label: TcsThresholds::get_tcs_narrative_label(edge.tcs_score).to_string(),
                    conditional_prob: edge.p_transition_conditional,
                    base_prob: edge.p_cs_base, // Added base probability
                },
                most_likely: PredictionPoint {
                    bias: likely.predicted_cs_bias.clone(),
                    score: likely.tcs_score,
                    label: TcsThresholds::get_tcs_narrative_label(likely.tcs_score).to_string(),
                    conditional_prob: likely.p_transition_conditional,
                    base_prob: likely.p_cs_base, // Added base probability
                },
                strongest_veto: PredictionPoint {
                    bias: veto.predicted_cs_bias.clone(),
                    score: veto.tcs_score,
                    label: TcsThresholds::get_tcs_narrative_label(veto.tcs_score).to_string(),
                    conditional_prob: veto.p_transition_conditional,
                    base_prob: veto.p_cs_base, // Added base probability
                },
            };
            final_summaries.insert(lookback, summary);
        }
    }
    final_summaries
}

// --- Phase 3: Cross-Order Comparison (FIXED FOR DYNAMIC NAMES) ---

/// Compares TCS 2nd Order results against 1st Order results to isolate the PS2 session's influence.
pub fn generate_cross_order_comparison(
    tcs_1st: &[Tcs1stOrderData],
    tcs_2nd: &[Tcs2ndOrderData],
) -> Vec<CrossOrderComparison> {
    
    // 1. Index 1st Order data (Key = (Lookback, Predicted_Bias))
    let tcs_1st_map: HashMap<(String, String), f64> = tcs_1st
        .iter()
        .map(|d| (
            (d.lookback_period.clone(), d.predicted_cs_bias.clone()), 
            d.tcs_score
        ))
        .collect();

    let mut comparisons = Vec::new();
    let delta_threshold = TcsThresholds::MIN_DELTA_FOR_SIGNIFICANCE;

    // Get dynamic session names from the first 2nd order record
    let (ps2_name, ps1_name, ps2_bias) = tcs_2nd.get(0).map(|d| {
        (d.current_ps2_session.as_str(), d.current_ps1_session.as_str(), d.current_ps2_bias.as_str())
    }).unwrap_or(("PS2", "PS1", "Bias"));


    // 2. Iterate through 2nd Order data and compare
    for d2 in tcs_2nd {
        let key = (d2.lookback_period.clone(), d2.predicted_cs_bias.clone());

        if let Some(&tcs1_score) = tcs_1st_map.get(&key) {
            
            let delta = d2.tcs_score - tcs1_score;
            
            // --- DYNAMIC NARRATIVE GENERATION ---
            let (narrative_label, explanation) = match delta {
                d if d >= delta_threshold => (
                    format!("🚀 {} Session Confirmation", ps2_name),
                    format!(
                        "The {} session ({}) significantly confirms this prediction, adding {:.2} points of TCS confidence.", 
                        ps2_name, ps2_bias, delta
                    )
                ),
                d if d <= -delta_threshold => (
                    format!("🛑 {} Session Veto/Contradiction", ps2_name),
                    format!(
                        "The {} session ({}) severely reduces confidence, subtracting {:.2} points. The {} context alone is misleading.",
                        ps2_name, ps2_bias, delta.abs(), ps1_name
                    )
                ),
                _ => (
                    format!("⚪ {} Session Neutral", ps2_name),
                    format!("The {} session had minimal statistical impact on this prediction (within {:.1} points).", ps2_name, delta_threshold)
                ),
            };

            comparisons.push(CrossOrderComparison {
                lookback_period: d2.lookback_period.clone(),
                predicted_cs_bias: d2.predicted_cs_bias.clone(),
                tcs_1st_order: tcs1_score,
                tcs_2nd_order: d2.tcs_score,
                delta,
                ps1_name: ps1_name.to_string(), // Store names for flexibility
                ps2_name: ps2_name.to_string(),
                ps2_bias: ps2_bias.to_string(),
                narrative_label: narrative_label, 
                explanation: explanation,
            });
        }
    }

    comparisons
}


// --- Phase 4: Final Report Assembly (FIXED FOR DYNAMIC NAMES) ---

/// Executes the full TCS analysis and compiles the results into a structured Markdown report.
pub fn generate_tcs_report(
    asset_id: &str,
    tcs1_data: Vec<Tcs1stOrderData>,
    tcs2_data: Vec<Tcs2ndOrderData>,
    analysis_date: &str,
) -> TcsFinalReport {

    // 1. Execute analysis components
    let tcs1_summaries = generate_single_metric_summary(tcs1_data.clone());
    let tcs2_summaries = HashMap::new(); // Placeholder for actual implementation
    let cross_order_comparisons = generate_cross_order_comparison(&tcs1_data, &tcs2_data);

    // 2. Get dynamic session names for the report narrative
    let (ps2_name, ps1_name) = tcs2_data.get(0).map(|d| {
        (d.current_ps2_session.as_str(), d.current_ps1_session.as_str())
    }).unwrap_or(("PS2", "PS1"));


    // 3. Assemble the Final Markdown Report (Narrative Generation)
    let mut report_markdown = format!("# TCS Predictive Analysis Report: {}\n\n", asset_id);
    report_markdown.push_str(&format!("**Analysis Date:** {}\n\n", analysis_date));
    report_markdown.push_str("--- \n\n");
    
    // --- Section A: Cross-Order Synthesis ---
    report_markdown.push_str(&format!("## 🔴 Cross-Order Synthesis: {} Session Influence\n\n", ps2_name));
    report_markdown.push_str(&format!(
        "This section shows the key difference between the **{} Context (1st Order)** and the **{} + {} Context (2nd Order)**, isolating the **{}** session's true impact.\n\n",
        ps1_name, ps1_name, ps2_name, ps2_name
    ));
    
    if cross_order_comparisons.is_empty() {
        report_markdown.push_str("No corresponding 1st Order data found for comparison.\n\n");
    } else {
        report_markdown.push_str(&format!("### Summary of {} Influence\n\n", ps2_name));

        for comp in &cross_order_comparisons {
            report_markdown.push_str(&format!("#### {} Lookback: Prediction for **{}**\n", comp.lookback_period, comp.predicted_cs_bias));
            report_markdown.push_str(&format!(
                "- **TCS 1st Order ({} Only):** `{:.2}`\n", 
                comp.ps1_name, comp.tcs_1st_order
            ));
            report_markdown.push_str(&format!(
                "- **TCS 2nd Order ({} + {}):** `{:.2}`\n", 
                comp.ps1_name, comp.ps2_name, comp.tcs_2nd_order
            ));
            report_markdown.push_str(&format!(
                "- **Delta ({} Influence):** `{:.2}`\n", 
                comp.ps2_name, comp.delta
            ));
            report_markdown.push_str(&format!(
                "- **Conclusion:** {} \n", 
                comp.narrative_label
            ));
            report_markdown.push_str(&format!("  *Explanation:* {}\n\n", comp.explanation));
        }
    }
    
    report_markdown.push_str("--- \n\n");
    
    // --- Section B: Key Takeaways (TCS 1st Order Summary) ---
    report_markdown.push_str(&format!("## 📈 TCS 1st Order Summary ({} Session)\n\n", ps1_name));
    report_markdown.push_str(&format!("This summary is based solely on the {} context (1st Order data). \n\n", ps1_name));
    
    // Iterate over the sorted lookbacks 
    let lookbacks = ["ALL", "1Y", "6M"]; 
    for period in lookbacks.iter() {
        if let Some(summary) = tcs1_summaries.get(*period) {
            report_markdown.push_str(&format!("### {} Lookback Period Analysis\n\n", period));
            
            // 1. Strongest Edge
            report_markdown.push_str("#### 1. The Strongest Edge (Highest Confidence)\n");
            report_markdown.push_str(&format!(
                "The most dominant signal is **{}** with a score of **{:.2}**. This registers as **{}**.\n\n",
                summary.strongest_edge.bias,
                summary.strongest_edge.score,
                summary.strongest_edge.label
            ));
            
            // 2. Most Likely Outcome
            report_markdown.push_str("#### 2. The Most Likely Outcome (Highest Conditional Probability)\n");
            report_markdown.push_str(&format!(
                "The highest conditional probability suggests **{}** is the most likely outcome, based on P_cond = **{:.2}** (P_base = {:.2}).\n\n",
                summary.most_likely.bias,
                summary.most_likely.conditional_prob,
                summary.most_likely.base_prob
            ));
            
            // 3. Strongest Veto
            report_markdown.push_str("#### 3. The Strongest Veto (Highest Failure Risk)\n");
            report_markdown.push_str(&format!(
                "The lowest confidence score, acting as a potential veto, is for **{}** at **{:.2}** ({}).\n\n",
                summary.strongest_veto.bias,
                summary.strongest_veto.score,
                summary.strongest_veto.label
            ));
        }
    }
    
    report_markdown.push_str("--- \n\n");
    report_markdown.push_str("## 📊 Detailed Data\n\n");
    report_markdown.push_str("*(Placeholder for detailed TCS data tables.)*");


    // 4. Return the final structured report
    TcsFinalReport {
        symbol: asset_id.to_string(),
        analysis_date: analysis_date.to_string(),
        tcs1_summaries,
        tcs2_summaries,
        cross_order_comparisons,
        final_markdown_report: report_markdown,
    }
}