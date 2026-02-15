
use crate::order_model::{Order, OrderSide};
use crate::order_executor::OrderExecutor;
use sqlx::PgPool;
use anyhow::Result;

pub struct OrderManager {
    db_pool: PgPool,
    executor: Box<dyn OrderExecutor>,
}

impl OrderManager {
    pub fn new(db_pool: PgPool, executor: Box<dyn OrderExecutor>) -> Self {
        Self { db_pool, executor }
    }

    // pub async fn place_market_order(&self, symbol: &str, side: OrderSide, quantity: f64) -> Result<Order> {
    //     let mut order = Order::new(symbol, side, quantity);

    //     // 1. Persist to Postgres as PENDING
    //     // (This uses your sqlx workspace dependency)
    //     sqlx::query!(
    //         "INSERT INTO orders (internal_id, symbol, side, quantity, status) VALUES ($1, $2, $3, $4, $5)",
    //         order.internal_id,
    //         order.symbol,
    //         format!("{:?}", order.side),
    //         order.quantity,
    //         format!("{:?}", order.status)
    //     )
    //     .execute(&self.db_pool)
    //     .await?;

    //     // 2. Execute via the Adapter (ZeroMQ/Wine)
    //     match self.executor.execute(&order).await {
    //         Ok(receipt) => {
    //             order.external_id = Some(receipt.ticket);
    //             order.status = crate::order_model::OrderStatus::Filled;
                
    //             // 3. Update status in DB
    //             sqlx::query!(
    //                 "UPDATE orders SET external_id = $1, status = 'Filled' WHERE internal_id = $2",
    //                 order.external_id,
    //                 order.internal_id
    //             )
    //             .execute(&self.db_pool)
    //             .await?;
    //         },
    //         Err(e) => {
    //             // Log failure...
    //             order.status = crate::order_model::OrderStatus::Rejected;
    //         }
    //     }

    //     Ok(order)
    // }
}