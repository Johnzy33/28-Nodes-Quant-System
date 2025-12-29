
use chrono::NaiveDate;
use std::collections::HashMap;
use std::hash::Hash;
use shared_models::models::{SessionContextData, DailyContextData,EightContextData
};


pub trait MetricsContext {
    type PatternKey: Eq + Hash + Clone;
    type OutcomeKey: Eq + Hash + Clone;

    fn get_date(&self) -> NaiveDate;
    fn get_asset_id(&self) -> String; 
    fn get_pattern_key(&self) -> Option<Self::PatternKey>;
    fn get_outcome_key(&self) -> Option<Self::OutcomeKey>;
}

// Sesion Context Data 
impl MetricsContext for SessionContextData {
    
    type PatternKey = (String, String, String, String);
    type OutcomeKey = String;

    fn get_asset_id(&self) -> String {
        self.asset_id.clone()
    }
    
    fn get_date(&self) -> NaiveDate {
        self.trading_date
    }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        
        if let (Some(ps2_n), Some(ps2_b), Some(ps1_n), Some(ps1_b)) = (
            &self.ps2_name, 
            &self.ps2_bias, 
            &self.ps1_name, 
            &self.ps1_bias
        ) {
            Some((ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        self.cs_bias.clone()
    }
}

// Sesison Base Rate Context Data
#[derive(Debug, Clone)]
pub struct BaseRateSessionContext {
    pub inner: SessionContextData,
}

impl MetricsContext for BaseRateSessionContext {
    // Key is now just the Session Name
    type PatternKey = String; 
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        // Just the session name (e.g., "NYAM")
        self.inner.cs_name.clone()
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        self.inner.cs_bias.clone()
    }
}

#[derive(Debug, Clone)]
pub struct BarBaseRateContext {
    pub inner: EightContextData,
}

// impl MetricsContext for BarBaseRateContext {
//     type PatternKey = bool; 
//     type OutcomeKey = String; 

//     fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
//     fn get_date(&self) -> NaiveDate { self.inner.trading_date }

//     fn get_pattern_key(&self) -> Option<Self::PatternKey> {
//         Some(true)
//     }

//     fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {

//         self.inner.cb_bias.clone()
//     }
// }

impl MetricsContext for BarBaseRateContext {
    // Just the Bar Number (e.g., 1, 2, or 3)
    type PatternKey = i32; 
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        // Just return the current bar number
        self.inner.cb_num 
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        // The bias for this specific bar
        self.inner.cb_bias.clone()
    }
}


// Daily Bias Base Rate Context Data
#[derive(Clone)]
pub struct DailyBaseRateContext {
    pub inner: DailyContextData,
}

impl MetricsContext for DailyBaseRateContext {
    // PatternKey is the Asset ID (e.g., "US30")
    type PatternKey = String; 
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        Some(self.inner.asset_id.clone())
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        self.inner.day_type.clone()
    }
}


// 2nd Order Transitional Condition Context Data
#[derive(Clone)]
pub struct Transition2ndOrderContext {
    pub inner: SessionContextData, 
    

}

impl MetricsContext for Transition2ndOrderContext {
   
    type PatternKey = (String, String, String, String); 
    type OutcomeKey = (String, String);

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
         if let (Some(ps2_name), Some(ps2_bias), Some(ps1_name), Some(ps1_bias)) = (
            &self.inner.ps2_name, 
            &self.inner.ps2_bias, 
            &self.inner.ps1_name, 
            &self.inner.ps1_bias
        ) {
            Some((ps2_name.clone(), ps2_bias.clone(), ps1_name.clone(), ps1_bias.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {

        if let (Some(cs_name), Some(cs_bias)) = (
            &self.inner.cs_name, 
            &self.inner.cs_bias
        ) {    
            if cs_bias != "Other" && cs_bias != "Consolidation-Other" {    
                Some((cs_name.clone(), cs_bias.clone())) 
            } else {   
                None
            }
        } else {        
            None
        }
    }
}

#[derive(Clone)]
pub struct Transition2ndOrderBarContext {
    pub inner: EightContextData, 
}

impl MetricsContext for Transition2ndOrderBarContext {
   
    type PatternKey = (i32, String, i32, String); 
    type OutcomeKey = (i32, String);

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
         if let (Some(pb2_num), Some(pb2_bias), Some(pb1_num), Some(pb1_bias)) = (
            &self.inner.pb2_num, 
            &self.inner.pb2_bias, 
            &self.inner.pb1_num, 
            &self.inner.pb1_bias
        ) {
            Some((pb2_num.clone(), pb2_bias.clone(), pb1_num.clone(), pb1_bias.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {

        if let (Some(cb_num), Some(cb_bias)) = (
            &self.inner.cb_num, 
            &self.inner.cb_bias
        ) {    
            if cb_bias != "Other" && cb_bias != "Consolidation-Other" {    
                Some((cb_num.clone(), cb_bias.clone())) 
            } else {   
                None
            }
        } else {        
            None
        }
    }
}



// 2nd Order Day Type Conditional Context Data
#[derive(Clone)]
pub struct DayType2ndOrderContext {
    pub inner: SessionContextData, 
    pub outcome:DailyContextData,
}


impl MetricsContext for DayType2ndOrderContext {
    
    type PatternKey = (String, String, String, String); 
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {

        if let (Some(ps2_name), Some(ps2_bias), Some(ps1_name), Some(ps1_bias)) = (
            &self.inner.ps2_name, 
            &self.inner.ps2_bias, 
            &self.inner.ps1_name, 
            &self.inner.ps1_bias
        ) {
            Some((ps2_name.clone(), ps2_bias.clone(), ps1_name.clone(), ps1_bias.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {

        self.outcome.day_type.clone()  
    }
}

#[derive(Clone)]
pub struct DayType2ndOrderBarContext {
    pub inner: EightContextData, 
    pub outcome:DailyContextData,
}


impl MetricsContext for DayType2ndOrderBarContext {
    
    type PatternKey = (i32, String, i32, String); 
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {

        if let (Some(pb2_num), Some(pb2_bias), Some(pb1_num), Some(pb1_bias)) = (
            &self.inner.pb2_num, 
            &self.inner.pb2_bias, 
            &self.inner.pb1_num, 
            &self.inner.pb1_bias
        ) {
            Some((pb2_num.clone(), pb2_bias.clone(), pb1_num.clone(), pb1_bias.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        
        self.outcome.day_type.clone()  
    }
}

// 2nd Order TCS Continuation Conditional Context Data
#[derive(Debug, Clone)]
pub struct SessionContinuationContext {
    pub inner: SessionContextData,
}

impl MetricsContext for SessionContinuationContext {
    // Key now has 5 elements: PS2_Name, PS2_Bias, PS1_Name, PS1_Bias, CS_Name
    type PatternKey = (String, String, String, String, String);
    type OutcomeKey = bool;

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        if let (Some(ps2n), Some(ps2b), Some(ps1n), Some(ps1b), Some(csn)) = (
            &self.inner.ps2_name, &self.inner.ps2_bias, 
            &self.inner.ps1_name, &self.inner.ps1_bias,
            &self.inner.cs_name // Include the target session name
        ) {
            Some((ps2n.clone(), ps2b.clone(), ps1n.clone(), ps1b.clone(), csn.clone()))
        } else { None }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        let ps1 = self.inner.ps1_bias.as_deref()?;
        let cs = self.inner.cs_bias.as_deref()?;
        Some(is_continuation(ps1, cs))
    }
}


pub fn is_continuation(prev_bias: &str, current_bias: &str) -> bool {
    let bullish_states = ["Bullish", "Bullish_Reversal", "Failed_Bullish"];
    let bearish_states = ["Bearish", "Bearish_Reversal", "Failed_Bearish"];

    if bullish_states.contains(&prev_bias) {
        bullish_states.contains(&current_bias)
    } else if bearish_states.contains(&prev_bias) {
        bearish_states.contains(&current_bias)
    } else {
        false
    }
}


// 2nd Order Session Extremes Context Data
#[derive(Debug, Clone)]
pub struct HighLowContext {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_bias: String,
    pub ps1_bias: String,
    pub ps2_name: String,
    pub ps1_name: String,
    pub session_extreme: String,     
    pub extreme_session_name: String, 
}


impl MetricsContext for HighLowContext {
    
    type PatternKey = (String, String, String, String, String);
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {

        Some((
            self.ps2_name.clone(), 
            self.ps2_bias.clone(),
            self.ps1_name.clone(), 
            self.ps1_bias.clone(),
            self.session_extreme.clone()
        ))
    }
    
    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        Some(self.extreme_session_name.clone())
    }
}

// Heper Function for upvoiting 
pub fn prepare_high_low_contexts(
    session_contexts: &[SessionContextData],
    daily_extremes: &[DailyContextData],
) -> Vec<HighLowContext> {
    
    
    let extremes_map: HashMap<(NaiveDate, String), &DailyContextData> = daily_extremes.iter()
        .map(|d| ((d.trading_date, d.asset_id.clone()), d))
        .collect();

    
    session_contexts.iter()
        .filter_map(|sc| {
            let key = (sc.trading_date, sc.asset_id.clone());
            let daily_view = extremes_map.get(&key)?;

            
            let ps2_n = sc.ps2_name.as_ref()?;
            let ps2_b = sc.ps2_bias.as_ref()?;
            let ps1_n = sc.ps1_name.as_ref()?;
            let ps1_b = sc.ps1_bias.as_ref()?;
            
            
            let high_context = HighLowContext {
                trading_date: sc.trading_date,
                asset_id: sc.asset_id.clone(),
                ps2_name: ps2_n.clone(),
                ps2_bias: ps2_b.clone(),
                ps1_name: ps1_n.clone(),
                ps1_bias: ps1_b.clone(),
                session_extreme: "High".to_string(),
                extreme_session_name: daily_view.high_session.clone(),
            };
            let low_context = HighLowContext {
                trading_date: sc.trading_date,
                asset_id: sc.asset_id.clone(),
                ps2_name: ps2_n.clone(),
                ps2_bias: ps2_b.clone(),
                ps1_name: ps1_n.clone(),
                ps1_bias: ps1_b.clone(),        
                session_extreme: "Low".to_string(),
                extreme_session_name: daily_view.low_session.clone(),
            };
            
            Some(vec![high_context, low_context])
        })
        .flatten()
        .collect()
}

#[derive(Clone)]
pub struct DayType2ndOrderSessionContext {
    pub inner: SessionContextData, 
    pub outcome: DailyContextData,
}

impl MetricsContext for DayType2ndOrderSessionContext {
    // Key: (PS2 Name, PS2 Bias, PS1 Name, PS1 Bias)
    type PatternKey = (String, String, String, String); 
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
         if let (Some(ps2_n), Some(ps2_b), Some(ps1_n), Some(ps1_b)) = (
            &self.inner.ps2_name, &self.inner.ps2_bias, 
            &self.inner.ps1_name, &self.inner.ps1_bias
        ) {
            Some((ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        self.outcome.day_type.clone()  
    }
}

// --  Core Metrics for Daily Models Begins Here ---




// Struct to hold the joined context necessary for 3rd order daily calculation
#[derive(Debug, Clone)]
pub struct Daily3rdOrderContext {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub pd2_bias: Option<String>,  
    pub pd1_bias: Option<String>,  
    pub pd1_dow: Option<String>,   
    pub c_day_bias: Option<String>, 
}

impl MetricsContext for Daily3rdOrderContext {
    
    type PatternKey = (String, String, String); 
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        
        if let (Some(pd2), Some(pd1), Some(dow)) = (
            &self.pd2_bias, 
            &self.pd1_bias, 
            &self.pd1_dow
        ) {
            Some((pd2.clone(), pd1.clone(), dow.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
       
        self.c_day_bias.clone()
            
            .filter(|bias| bias != "Other" && bias != "Consolidation-Other") 
    }
}



// Context struct that holds the joined data required for the 3rd order trend rate
#[derive(Debug, Clone)]
pub struct DailyTrendContinuationContext {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    
    // Pattern Inputs
    pub pd2_bias: Option<String>,
    pub pd1_bias: Option<String>,
    pub pd1_dow: Option<String>, 
    
    // Outcome Inputs
    pub c_day_bias: String, // The actual 7-level outcome bias of the current day
    pub continuation_outcome: String, // The binary (Continuation/Reversal) flag
}

impl MetricsContext for DailyTrendContinuationContext {
    // Pattern is the input key: (PD2_Bias, PD1_Bias, PD1_DOW)
    type PatternKey = (String, String, String); 
    
    // Outcome is now a composite key: (C_Day_Bias, Continuation_Flag)
    type OutcomeKey = (String, String); 

    fn get_asset_id(&self) -> String { self.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        if let (Some(pd2), Some(pd1), Some(dow)) = (
            &self.pd2_bias, 
            &self.pd1_bias, 
            &self.pd1_dow
        ) {
            Some((pd2.clone(), pd1.clone(), dow.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        
        Some((self.c_day_bias.clone(), self.continuation_outcome.clone()))
    }
}

// pub fn prepare_dtcr_contexts(
//     daily_contexts: &[DailyContextData]
// ) -> Vec<DailyTrendContinuationContext> {
    
//     let mut prepared_contexts: Vec<DailyTrendContinuationContext> = Vec::new();
    
//     for i in 2..daily_contexts.len() {
        
//         let current_day = &daily_contexts[i];
//         let pd1 = &daily_contexts[i - 1]; 
//         let pd2 = &daily_contexts[i - 2]; 
        
//         let daily_bias = |d: &DailyContextData| -> Option<String> {
//             map_daily_bias(d.day_type.clone(), d.consolidation_subtype.clone())
//         };
        
//         let pd2_bias = daily_bias(pd2);
//         let pd1_bias = daily_bias(pd1);
//         let c_day_bias = daily_bias(current_day); // We need this for the outcome check
//         let pd1_dow = pd1.trading_date.format("%a").to_string().into(); 
        
//         // Ensure ALL pattern components and the current outcome are valid 7-level biases
//         if let (Some(ref pd2_b), Some(ref pd1_b), Some(ref c_day_b)) = (&pd2_bias, &pd1_bias, &c_day_bias) {
    
//         let continuation_outcome = if iss_continuation(pd1_b, c_day_b) {
//             "Continuation".to_string()
//         } else {
//             "Reversal".to_string()
//         };

//         let context = DailyTrendContinuationContext {
//             trading_date: current_day.trading_date,
//             asset_id: current_day.asset_id.clone(),
            
//             pd2_bias: pd2_bias.clone(),
//             pd1_bias: pd1_bias.clone(),
//             pd1_dow: pd1_dow, 
            
//             c_day_bias: c_day_b.clone(), // <-- Store the 7-level bias here
//             continuation_outcome,
//         };
                
//                 prepared_contexts.push(context);
//             }
//     }
    
//     prepared_contexts
// }

pub fn prepare_dtcr_contexts(
    daily_contexts: &[DailyContextData]
) -> Vec<DailyTrendContinuationContext> {
    
    let mut prepared_contexts: Vec<DailyTrendContinuationContext> = Vec::new();
    
    for i in 2..daily_contexts.len() {
        
        let current_day = &daily_contexts[i];
        let pd1 = &daily_contexts[i - 1]; 
        let pd2 = &daily_contexts[i - 2]; 
        
        let daily_bias = |d: &DailyContextData| -> Option<String> {
            d.day_type.clone()
        };
        
        // 7-level bias strings (Option<String>)
        let pd2_bias_7l = daily_bias(pd2);
        let pd1_bias_7l = daily_bias(pd1); // Source of the 7-level string
        let c_day_bias_7l = daily_bias(current_day);
        let pd1_dow = pd1.trading_date.format("%a").to_string().into(); 
        
        // Use references to the inner String for the logic check
        if let (Some(ref pd2_b), Some(ref pd1_b), Some(ref c_day_b)) = 
            (&pd2_bias_7l, &pd1_bias_7l, &c_day_bias_7l) {
            
            // 1. Logic Check: Use the reference variables for the conditional check
            let continuation_outcome = if iss_continuation(pd1_b, c_day_b) {
                "Continuation".to_string()
            } else {
                "Reversal".to_string()
            };

            // 2. Data Persistence: Create a new, independent copy of the pattern key data
            //    This is the core change to break the dependency on the 'pd1_b' reference
            let pd1_bias_key = pd1_bias_7l.clone();
            let pd2_bias_key = pd2_bias_7l.clone();
            let c_day_bias_key = c_day_bias_7l.clone().unwrap();

            let context = DailyTrendContinuationContext {
                trading_date: current_day.trading_date,
                asset_id: current_day.asset_id.clone(),
                
                // Use the new, independent Option<String> clones for the context fields
                pd2_bias: pd2_bias_key,
                pd1_bias: pd1_bias_key,
                pd1_dow: pd1_dow, 
                
                c_day_bias: c_day_bias_key,
                continuation_outcome,
            };
            
            prepared_contexts.push(context);
        }
    }
    
    prepared_contexts
}

fn iss_continuation(pd1_bias: &str, c_day_bias: &str) -> bool {

    // These states define the 'family' of directional biases
    let bullish_states = &["Bullish", "Bullish_Reversal" , "Failed_Bullish"]; 
    let bearish_states = &["Bearish", "Bearish_Reversal", "Failed_Bearish"];

    // Match PD1 bias to its family and check if the current day (C_day_bias) belongs to the same family
    match pd1_bias {
        s if bullish_states.contains(&s) => bullish_states.contains(&c_day_bias),
        s if bearish_states.contains(&s) => bearish_states.contains(&c_day_bias),
        _ => false, // No continuation possible if PD1 bias is 'Other' or 'Consolidation'
    }
}


// fn is_continuation(pd1_bias: &str, c_day_bias: &str) -> bool {

//     // These states define the 'family' of directional biases
//     let bullish_states = &["Bullish", "Bullish_Reversal", "Failed_Bullish"];
//     let bearish_states = &["Bearish", "Bearish_Reversal", "Failed_Bearish"];

//     // Match PD1 bias to its family and check if the current day (C_day_bias) belongs to the same family
//     match pd1_bias {
//         s if bullish_states.contains(&s) => bullish_states.contains(&c_day_bias),
//         s if bearish_states.contains(&s) => bearish_states.contains(&c_day_bias),
//         _ => false, // No continuation possible if PD1 bias is 'Other' or 'Consolidation'
//     }
// }

pub fn prepare_daily_3rd_order_contexts(
    daily_contexts: &[DailyContextData]
) -> Vec<Daily3rdOrderContext> {
    
    let mut prepared_contexts: Vec<Daily3rdOrderContext> = Vec::new();
    
    
    for i in 2..daily_contexts.len() {
        
        let current_day = &daily_contexts[i];
        let pd1 = &daily_contexts[i - 1]; 
        let pd2 = &daily_contexts[i - 2]; 
        
        
        
        let daily_bias = |d: &DailyContextData| -> Option<String> {
            d.day_type.clone()
        };
        
        let pd2_bias = daily_bias(pd2);
        let pd1_bias = daily_bias(pd1);
        let c_day_bias = daily_bias(current_day);
        let pd1_dow = pd1.trading_date.format("%a").to_string().into(); 
        
        let context = Daily3rdOrderContext {
            trading_date: current_day.trading_date,
            asset_id: current_day.asset_id.clone(),
            pd2_bias,
            pd1_bias,
            pd1_dow, 
            c_day_bias,
        };
        
        prepared_contexts.push(context);
    }
    
    prepared_contexts
}


#[derive(Clone)]
pub struct SessionToBar2ndOrderContext {
    pub inner: SessionContextData,
    pub target_bar: EightContextData,
}

impl MetricsContext for SessionToBar2ndOrderContext {
    // Key: (PS2_Name, PS2_Bias, PS1_Name, PS1_Bias)
    type PatternKey = (String, String, String, String); 
    // Outcome: "BarNum:Bias" so you can manually verify the mapping
    type OutcomeKey = String; 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        if let (Some(ps2_n), Some(ps2_b), Some(ps1_n), Some(ps1_b)) = (
            &self.inner.ps2_name, &self.inner.ps2_bias, 
            &self.inner.ps1_name, &self.inner.ps1_bias
        ) {
            Some((ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        // Formats as "3:Bullish" for easy manual checking
        if let (Some(num), Some(bias)) = (self.target_bar.cb_num, &self.target_bar.cb_bias) {
            Some(format!("{}:{}", num, bias))
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct BarType2ndOrderSessionContext {
    pub inner: SessionContextData, 
    pub bar_outcome: EightContextData,
}

impl MetricsContext for BarType2ndOrderSessionContext {
    // Key: (PS2 Name, PS2 Bias, PS1 Name, PS1 Bias)
    type PatternKey = (String, String, String, String); 
    // Outcome: (Bar Number, Bar Bias)
    type OutcomeKey = (i32, String); 

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
         if let (Some(ps2_n), Some(ps2_b), Some(ps1_n), Some(ps1_b)) = (
            &self.inner.ps2_name, &self.inner.ps2_bias, 
            &self.inner.ps1_name, &self.inner.ps1_bias
        ) {
            Some((ps2_n.clone(), ps2_b.clone(), ps1_n.clone(), ps1_b.clone()))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        if let (Some(cb_num), Some(cb_bias)) = (self.bar_outcome.cb_num, &self.bar_outcome.cb_bias) {
            if cb_bias != "Other" && cb_bias != "Consolidation-Other" {
                return Some((cb_num, cb_bias.clone()));
            }
        }
        None
    }
}

#[derive(Clone)]
// --- BAR CONTINUATION CONTEXT ---
pub struct BarContinuationContext {
    pub inner: EightContextData,
}

impl MetricsContext for BarContinuationContext {
    // Key: (PB2_Num, PB2_Bias, PB1_Num, PB1_Bias, CB_Num)
    type PatternKey = (i32, String, i32, String, i32);
    type OutcomeKey = bool;

    fn get_asset_id(&self) -> String { self.inner.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.inner.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        if let (Some(pb2n), Some(pb2b), Some(pb1n), Some(pb1b), Some(cbn)) = (
            self.inner.pb2_num, &self.inner.pb2_bias,
            self.inner.pb1_num, &self.inner.pb1_bias,
            self.inner.cb_num
        ) {
            Some((pb2n, pb2b.clone(), pb1n, pb1b.clone(), cbn))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        let pb1 = self.inner.pb1_bias.as_deref()?;
        let cb = self.inner.cb_bias.as_deref()?;
        Some(is_continuation(pb1, cb))
    }
}


// --- DAILY CONTINUATION CONTEXT ---
#[derive(Clone)]
// Assuming Daily raw data is NOT pre-joined, we use a wrapper for the sliding window
pub struct DailyContinuationContext {
    pub pd2: DailyContextData,
    pub pd1: DailyContextData,
    pub cd: DailyContextData,
}

impl MetricsContext for DailyContinuationContext {
    // Key: (PD2_Bias, PD1_Bias, PD1_DOW)
    type PatternKey = (String, String, String);
    type OutcomeKey = bool;

    fn get_asset_id(&self) -> String { self.cd.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.cd.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        use chrono::Datelike;
        let dow = self.pd1.trading_date.weekday().to_string();
        
        if let (Some(pd2b), Some(pd1b)) = (&self.pd2.day_type, &self.pd1.day_type) {
            Some((pd2b.clone(), pd1b.clone(), dow))
        } else {
            None
        }
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        let pd1 = self.pd1.day_type.as_deref()?;
        let cd = self.cd.day_type.as_deref()?;
        Some(is_continuation(pd1, cd))
    }
}


#[derive(Clone)]
pub struct SessionExtremeContext {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub ps2_name: String,
    pub ps2_bias: String,
    pub ps1_name: String,
    pub ps1_bias: String,
    pub extreme_type: String,         // "High" or "Low"
    pub extreme_session_name: String, // The session that actually held it
}

impl MetricsContext for SessionExtremeContext {
    type PatternKey = (String, String, String, String, String); // ps2n, ps2b, ps1n, ps1b, extreme_type
    type OutcomeKey = String; // extreme_session_name

    fn get_asset_id(&self) -> String { self.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        Some((
            self.ps2_name.clone(), self.ps2_bias.clone(),
            self.ps1_name.clone(), self.ps1_bias.clone(),
            self.extreme_type.clone()
        ))
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        Some(self.extreme_session_name.clone())
    }
}

pub fn prepare_session_extreme_contexts(
    session_contexts: &[SessionContextData],
    daily_extremes: &[DailyContextData],
) -> Vec<SessionExtremeContext> {
    // 1. O(1) Map for daily extremes
    let daily_map: HashMap<(NaiveDate, String), &DailyContextData> = daily_extremes.iter()
        .map(|d| ((d.trading_date, d.asset_id.clone()), d))
        .collect();

    session_contexts.iter().filter_map(|sc| {
        let daily = daily_map.get(&(sc.trading_date, sc.asset_id.clone()))?;
        
        let ps2n = sc.ps2_name.as_ref()?;
        let ps2b = sc.ps2_bias.as_ref()?;
        let ps1n = sc.ps1_name.as_ref()?;
        let ps1b = sc.ps1_bias.as_ref()?;

        // We generate TWO contexts per session record: one for High, one for Low
        let mut contexts = Vec::with_capacity(2);
        
        contexts.push(SessionExtremeContext {
            trading_date: sc.trading_date,
            asset_id: sc.asset_id.clone(),
            ps2_name: ps2n.clone(), ps2_bias: ps2b.clone(),
            ps1_name: ps1n.clone(), ps1_bias: ps1b.clone(),
            extreme_type: "High".to_string(),
            extreme_session_name: daily.high_session.clone(),
        });

        contexts.push(SessionExtremeContext {
            trading_date: sc.trading_date,
            asset_id: sc.asset_id.clone(),
            ps2_name: ps2n.clone(), ps2_bias: ps2b.clone(),
            ps1_name: ps1n.clone(), ps1_bias: ps1b.clone(),
            extreme_type: "Low".to_string(),
            extreme_session_name: daily.low_session.clone(),
        });

        Some(contexts)
    })
    .flatten()
    .collect()
}

#[derive(Clone)]
pub struct SessionExtremeBaseContext {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub extreme_type: String,         // "High" or "Low"
    pub extreme_session_name: String, // The session that held it (Outcome)
}

impl MetricsContext for SessionExtremeBaseContext {
    type PatternKey = String; // extreme_type ("High" or "Low")
    type OutcomeKey = String; // extreme_session_name ("LN", "AS", etc.)

    fn get_asset_id(&self) -> String { self.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.trading_date }

    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        Some(self.extreme_type.clone())
    }

    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> {
        Some(self.extreme_session_name.clone())
    }
}

#[derive(Clone)]
// --- BAR EXTREME BASE CONTEXT ---
pub struct BarExtremeBaseContext {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub extreme_type: String,   // "High" or "Low"
    pub extreme_cb_num: i32,    // The bar number (1, 2, or 3) from DailyContextData
}

impl MetricsContext for BarExtremeBaseContext {
    type PatternKey = String; // extreme_type
    type OutcomeKey = i32;    // extreme_cb_num

    fn get_asset_id(&self) -> String { self.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.trading_date }
    fn get_pattern_key(&self) -> Option<Self::PatternKey> { Some(self.extreme_type.clone()) }
    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> { Some(self.extreme_cb_num) }
}

#[derive(Clone)]
// --- BAR EXTREME OUTCOME CONTEXT ---
pub struct BarExtremeContext {
    pub trading_date: NaiveDate,
    pub asset_id: String,
    pub pb2_num: i32, pub pb2_bias: String,
    pub pb1_num: i32, pub pb1_bias: String,
    pub extreme_type: String,
    pub extreme_cb_num: i32,
}

impl MetricsContext for BarExtremeContext {
    type PatternKey = (i32, String, i32, String, String); // pb2n, pb2b, pb1n, pb1b, type
    type OutcomeKey = i32; // extreme_cb_num

    fn get_asset_id(&self) -> String { self.asset_id.clone() }
    fn get_date(&self) -> NaiveDate { self.trading_date }
    fn get_pattern_key(&self) -> Option<Self::PatternKey> {
        Some((self.pb2_num, self.pb2_bias.clone(), self.pb1_num, self.pb1_bias.clone(), self.extreme_type.clone()))
    }
    fn get_outcome_key(&self) -> Option<Self::OutcomeKey> { Some(self.extreme_cb_num) }
}

pub fn prepare_bar_extreme_contexts(
    bar_data: &[EightContextData],
    daily_extremes: &[DailyContextData],
) -> Vec<BarExtremeContext> {
    // 1. Create a map for O(1) daily lookup
    let daily_map: HashMap<(NaiveDate, String), &DailyContextData> = daily_extremes.iter()
        .map(|d| ((d.trading_date, d.asset_id.clone()), d))
        .collect();

    bar_data.iter().filter_map(|b| {
        let daily = daily_map.get(&(b.trading_date, b.asset_id.clone()))?;
        
        // Ensure we have the pattern data
        let pb2n = b.pb2_num?;
        let pb2b = b.pb2_bias.as_ref()?;
        let pb1n = b.pb1_num?;
        let pb1b = b.pb1_bias.as_ref()?;

        // Create two contexts: one for High outcome, one for Low outcome
        Some(vec![
            BarExtremeContext {
                trading_date: b.trading_date,
                asset_id: b.asset_id.clone(),
                pb2_num: pb2n, pb2_bias: pb2b.clone(),
                pb1_num: pb1n, pb1_bias: pb1b.clone(),
                extreme_type: "High".to_string(),
                extreme_cb_num: daily.high_bar, // From DB
            },
            BarExtremeContext {
                trading_date: b.trading_date,
                asset_id: b.asset_id.clone(),
                pb2_num: pb2n, pb2_bias: pb2b.clone(),
                pb1_num: pb1n, pb1_bias: pb1b.clone(),
                extreme_type: "Low".to_string(),
                extreme_cb_num: daily.low_bar, // From DB
            }
        ])
    })
    .flatten()
    .collect()
}