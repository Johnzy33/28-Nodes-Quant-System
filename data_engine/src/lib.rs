pub mod ingestion;
pub mod config;
pub mod csv_reader;

pub use ingestion::ingest_from_csv;
pub use config::ProducerConfig;
// pub use csv_reader::CsvRecord; // <-- REMOVE THIS LINE (it's no longer defined)

// Add the new specific struct names so they can be accessed externally
pub use csv_reader::{CsvRecordStandard, CsvRecordBracketed};