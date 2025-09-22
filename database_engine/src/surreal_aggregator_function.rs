
use anyhow::{Result, Context};
use crate::DB;

pub async fn define_surreal_functions(db: &DB) -> Result<()> {
    println!("  - Defining SurrealQL functions...");
    let define_function_query = "
        DEFINE FUNCTION fn::session_from_timestamp($ts: datetime) -> string {
            LET $hour = time::hour($ts);
            RETURN IF $hour >= 0 AND $hour <= 7 THEN 'AS'
                ELSE IF $hour >= 8 AND $hour <= 14 THEN 'LN'
                ELSE IF $hour >= 15 AND $hour <= 17 THEN 'NYAM'
                ELSE IF $hour >= 18 AND $hour <= 19 THEN 'NYL'
                ELSE IF $hour >= 20 AND $hour <= 23 THEN 'NYPM'
                ELSE 'None'
                END;
};

    ";
    db.query(define_function_query)
        .await
        .context("Failed to define SurrealQL function")?;

    println!("  - SurrealQL functions defined. ✅");
    Ok(())
}

