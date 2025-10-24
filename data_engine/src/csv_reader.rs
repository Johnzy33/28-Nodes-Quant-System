
use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct CsvRecord {
    // These fields must match the headers in your exported CSV file
    #[serde(rename = "Date")] 
    pub date: String,
    #[serde(rename = "Time")] 
    pub time: String,
    
    #[serde(rename = "Open")]
    pub open: f64,
    #[serde(rename = "High")]
    pub high: f64,
    #[serde(rename = "Low")]
    pub low: f64,
    #[serde(rename = "Close")]
    pub close: f64,
    #[serde(rename = "Volume")]
    pub volume: f64,
}