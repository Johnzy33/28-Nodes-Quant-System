use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use tokio_postgres::{NoTls, Error};
use serde::{Deserialize};

#[derive(Deserialize, Debug)]
struct MarketData {
    symbol: String,
    price: f64,
    volume: i64,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    //
    // Kafka Consumer Setup with Container IP Address
    //
    let kafka_brokers = "<KAFKA_IP_ADDRESS>:9092"; // Use the IP you found here!
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "market-data-consumer")
        .set("bootstrap.servers", kafka_brokers)
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Consumer creation failed");

    consumer
        .subscribe(&["market_data_topic"])
        .expect("Failed to subscribe to topic");

    //
    // TimescaleDB (Postgres) Connection Setup
    //
    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres dbname=timescale password=password",
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {}", e);
        }
    });

    //
    // Main Data Ingestion Loop
    //
    println!("Starting data ingestion from Kafka...");
    loop {
        let message = consumer.recv().await.unwrap();
        let payload = message.payload().expect("Failed to get message payload");
        let data: MarketData = serde_json::from_slice(payload).unwrap();

        let statement = client.prepare(
            "INSERT INTO market_data (time, symbol, price, volume) VALUES (NOW(), $1, $2, $3)"
        ).await?;

        client.execute(&statement, &[&data.symbol, &data.price, &data.volume]).await?;

        consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async).unwrap();
    }
}