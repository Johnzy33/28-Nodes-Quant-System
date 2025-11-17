pub mod ingestion;
pub mod csv_reader;

pub use ingestion::ingest_from_csv;

// Add the new specific struct names so they can be accessed externally
pub use csv_reader::{CsvRecordStandard, CsvRecordBracketed};