pub mod ingestion;
pub mod config;
pub mod csv_reader;

pub use ingestion::ingest_from_csv;
pub use config::ProducerConfig;
pub use csv_reader::CsvRecord;