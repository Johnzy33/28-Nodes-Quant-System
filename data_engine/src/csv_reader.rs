use serde::Deserialize;

// 1. Struct for files with simple headers (e.g., "Date", "Time", "Volume")
#[derive(Debug, Deserialize)]
pub struct CsvRecordStandard {
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
    #[serde(rename = "Volume")] // Standard header is "Volume"
    pub volume: f64,
}

// 2. Struct for files with bracketed headers (e.g., "<DATE>", "<TICKVOL>")
#[derive(Debug, Deserialize)]
pub struct CsvRecordBracketed {
    #[serde(rename = "<DATE>")] 
    pub date: String,
    #[serde(rename = "<TIME>")] 
    pub time: String,
    #[serde(rename = "<OPEN>")]
    pub open: f64,
    #[serde(rename = "<HIGH>")]
    pub high: f64,
    #[serde(rename = "<LOW>")]
    pub low: f64,
    #[serde(rename = "<CLOSE>")]
    pub close: f64,
    #[serde(rename = "<TICKVOL>")] // Bracketed header is "<TICKVOL>"
    pub volume: f64,
    // Include extra columns to match the file structure
    #[serde(rename = "<VOL>")]
    pub extra_vol: f64,
    #[serde(rename = "<SPREAD>")]
    pub spread: f64,
}