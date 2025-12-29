use serde::{Serialize, Deserialize};
use sqlx::prelude::FromRow;




#[derive(Debug, Clone, Serialize)]
pub struct SessionBaseRateML {
    pub asset_id: String,
    pub session_name: String,
    pub session_bias: String,

    // 6 Month
    pub p_6m: f64,
    pub count_6m: f64,
    pub total_6m: f64,

    // 1 Year
    pub p_1y: f64,
    pub count_1y: f64,
    pub total_1y: f64,

    // All History
    pub p_all: f64,
    pub count_all: f64,
    pub total_all: f64,
}

// For Bar-level bias (e.g., specific 8h candle outcomes)
#[derive(Debug, Clone, Serialize)]
pub struct BarBaseRateML {
    pub asset_id: String,
    pub bar_num: i32,
    pub bar_bias: String,
    // 6M, 1Y, and ALL fields
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

// For Daily-level bias (e.g., Daily candle outcomes)
#[derive(Debug, Clone, Serialize)]
pub struct DailyBaseRateML {
    pub asset_id: String,
    pub daily_bias: String,
    // 6M, 1Y, and ALL fields
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}


#[derive(Debug, Clone, Serialize)]
pub struct Transition2ndOrderML {
    pub asset_id: String,
    // Sequence: ps2_name, ps2_bias, ps1_name, ps1_bias
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    // Outcome: cs_name, cs_bias
    pub cs_name: String,
    pub cs_bias: String,

    // 6 Month Features
    pub p_6m: f64,
   // pub lift_6m: f64,
    pub count_6m: f64,
    pub total_6m: f64,

    // 1 Year Features
    pub p_1y: f64,
    //pub lift_1y: f64,
    pub count_1y: f64,
    pub total_1y: f64,

    // All History Features
    pub p_all: f64,
    //pub lift_all: f64,
    pub count_all: f64,
    pub total_all: f64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Tcs2ndOrderML {
    pub asset_id: String,
    pub ps2_name: String, pub ps2_bias: String,
    pub ps1_name: String, pub ps1_bias: String,
    pub cs_name: String, pub cs_bias: String,

    // 6 Month Vector
    pub p_cond_6m: f64,
    pub p_base_6m: f64,
    pub tcs_score_6m: f64,

    // 1 Year Vector
    pub p_cond_1y: f64,
    pub p_base_1y: f64,
    pub tcs_score_1y: f64,

    // All History Vector
    pub p_cond_all: f64,
    pub p_base_all: f64,
    pub tcs_score_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BarTransition2ndOrderML {
    pub asset_id: String,
    pub pb2_num: i32, pub pb2_bias: String,
    pub pb1_num: i32, pub pb1_bias: String,
    pub cb_num: i32, pub cb_bias: String,

    // Raw probabilities
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stcs2ndOrderML {
    pub asset_id: String,
    pub pb2_num: i32, pub pb2_bias: String,
    pub pb1_num: i32, pub pb1_bias: String,
    pub cb_num: i32, pub cb_bias: String,

    // 6 Month Vector
    pub p_cond_6m: f64, pub p_base_6m: f64, pub stcs_score_6m: f64,
    // 1 Year Vector
    pub p_cond_1y: f64, pub p_base_1y: f64, pub stcs_score_1y: f64,
    // All History Vector
    pub p_cond_all: f64, pub p_base_all: f64, pub stcs_score_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BarToDailyOutcomeML {
    pub asset_id: String,
    pub pb2_num: i32, pub pb2_bias: String,
    pub pb1_num: i32, pub pb1_bias: String,
    pub day_type: String,

    // Raw probabilities across all 3 intervals
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BarToDailyLiftML {
    pub asset_id: String,
    pub pb2_num: i32, pub pb2_bias: String,
    pub pb1_num: i32, pub pb1_bias: String,
    pub day_type: String,

    // Lift Vectors (Conditional / Daily Base Rate)
    pub p_cond_6m: f64, pub p_base_6m: f64, pub lift_6m: f64,
    pub p_cond_1y: f64, pub p_base_1y: f64, pub lift_1y: f64,
    pub p_cond_all: f64, pub p_base_all: f64, pub lift_all: f64,
}


#[derive(Debug, Clone, Serialize)]
pub struct SessionToDailyOutcomeML {
    pub asset_id: String,
    pub ps2_name: String, pub ps2_bias: String,
    pub ps1_name: String, pub ps1_bias: String,
    pub day_type: String,
    // Probabilities for 6M, 1Y, ALL
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionToDailyLiftML {
    pub asset_id: String,
    pub ps2_name: String, pub ps2_bias: String,
    pub ps1_name: String, pub ps1_bias: String,
    pub day_type: String,
    // Lift vectors
    pub p_cond_6m: f64, pub p_base_6m: f64, pub lift_6m: f64,
    pub p_cond_1y: f64, pub p_base_1y: f64, pub lift_1y: f64,
    pub p_cond_all: f64, pub p_base_all: f64, pub lift_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionToBarOutcomeML {
    pub asset_id: String,
    pub ps2_name: String, pub ps2_bias: String,
    pub ps1_name: String, pub ps1_bias: String,
    pub cb_num: i32, pub cb_bias: String,
    
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionToBarLiftML {
    pub asset_id: String,
    pub ps2_name: String, pub ps2_bias: String,
    pub ps1_name: String, pub ps1_bias: String,
    pub cb_num: i32, pub cb_bias: String,

    pub p_cond_6m: f64, pub p_base_6m: f64, pub lift_6m: f64,
    pub p_cond_1y: f64, pub p_base_1y: f64, pub lift_1y: f64,
    pub p_cond_all: f64, pub p_base_all: f64, pub lift_all: f64,
}


#[derive(Debug, Clone, Serialize)]
pub struct DailyOutcome3rdOrderML {
    pub asset_id: String,
    pub pd2_bias: String,
    pub pd1_bias: String,
    pub pd1_dow: String,
    pub day_type: String, // Outcome
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyOutcome3rdOrderLiftML {
    pub asset_id: String,
    pub pd2_bias: String,
    pub pd1_bias: String,
    pub pd1_dow: String,
    pub day_type: String,
    pub p_cond_6m: f64, pub p_base_6m: f64, pub lift_6m: f64,
    pub p_cond_1y: f64, pub p_base_1y: f64, pub lift_1y: f64,
    pub p_cond_all: f64, pub p_base_all: f64, pub lift_all: f64,
}

// Session Continuation: PS2 + PS1 -> CS (Name + Continuation Prob)
#[derive(Debug, Clone, Serialize)]
pub struct SessionContinuationML {
    pub asset_id: String,
    pub ps2_name: String, pub ps2_bias: String,
    pub ps1_name: String, pub ps1_bias: String,
    pub cs_name: String, // The "Target" Session
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

// Bar Continuation: PB2 + PB1 -> CB (Num + Continuation Prob)
#[derive(Debug, Clone, Serialize)]
pub struct BarContinuationML {
    pub asset_id: String,
    pub pb2_num: i32, pub pb2_bias: String,
    pub pb1_num: i32, pub pb1_bias: String,
    pub cb_num: i32, // The "Target" Bar
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

// Daily Continuation: PD2 + PD1 + DOW -> Prediction of Continuation
#[derive(Debug, Clone, Serialize)]
pub struct DailyContinuationML {
    pub asset_id: String,
    pub pd2_bias: String,
    pub pd1_bias: String,
    pub pd1_dow: String, // Context of the day
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}


// 1. High/Low Base Rate (Which session usually holds the extreme?)
#[derive(Debug, Clone, Serialize)]
pub struct SessionExtremeBaseRateML {
    pub asset_id: String,
    pub session_name: String,   // AS, LN, NYAM, etc.
    pub extreme_type: String,   // "High" or "Low"
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

// 2. High/Low Conditional (PS2 + PS1 -> Which session forms the extreme?)
#[derive(Debug, Clone, Serialize)]
pub struct SessionExtremeOutcomeML {
    pub asset_id: String,
    pub ps2_name: String, pub ps2_bias: String,
    pub ps1_name: String, pub ps1_bias: String,
    pub extreme_type: String,         // "High" or "Low"
    pub extreme_session_name: String, // The "Outcome" (e.g., NYAM formed the High)
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionExtremeLiftML {
    pub asset_id: String,
    pub ps2_name: String, pub ps2_bias: String,
    pub ps1_name: String, pub ps1_bias: String,
    pub extreme_type: String,         // "High" or "Low"
    pub extreme_session_name: String, // The "Outcome" (e.g., NYAM formed the High)
    pub p_cond_6m: f64, pub p_base_6m: f64, pub lift_6m: f64,
    pub p_cond_1y: f64, pub p_base_1y: f64, pub lift_1y: f64,
    pub p_cond_all: f64, pub p_base_all: f64, pub lift_all: f64,
}

// 1. Base Rate: Global probability of a specific Bar Number being the High/Low
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarExtremeBaseRateML {
    pub asset_id: String,
    pub cb_num: i32,            // 1, 2, or 3
    pub extreme_type: String,   // "High" or "Low"
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

// 2. Outcome: Probability of which Bar Number is the extreme based on PB2/PB1 patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarExtremeOutcomeML {
    pub asset_id: String,
    pub pb2_num: i32, 
    pub pb2_bias: String,
    pub pb1_num: i32, 
    pub pb1_bias: String,
    pub extreme_type: String,    // "High" or "Low"
    pub extreme_cb_num: i32,     // The outcome: which bar actually held it
    pub p_6m: f64, pub count_6m: f64, pub total_6m: f64,
    pub p_1y: f64, pub count_1y: f64, pub total_1y: f64,
    pub p_all: f64, pub count_all: f64, pub total_all: f64,
}

// 3. Lift: The statistical edge of the pattern over the base rate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarExtremeLiftML {
    pub asset_id: String,
    pub pb2_num: i32, 
    pub pb2_bias: String,
    pub pb1_num: i32, 
    pub pb1_bias: String,
    pub extreme_type: String,
    pub extreme_cb_num: i32,
    pub p_cond_6m: f64, pub p_base_6m: f64, pub lift_6m: f64,
    pub p_cond_1y: f64, pub p_base_1y: f64, pub lift_1y: f64,
    pub p_cond_all: f64, pub p_base_all: f64, pub lift_all: f64,
}