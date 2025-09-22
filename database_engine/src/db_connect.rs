use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::Surreal;
use anyhow::{Result, Context};
use surrealdb::opt::auth::Root;
use crate::schema;
//use crate::DB;

pub type DB = Surreal<Client>;

/// Connects to the SurrealDB server and signs in.
pub async fn connect() -> Result<DB> {
   
    let db = Surreal::new::<Ws>("127.0.0.1:8000").await.context("Failed to connect to SurrealDB")?;
    
    // Sign in to the database using the Credentials struct
   db.signin(Root {username: "root", password: "root",}).await.context("Failed to sigin")?;
    
    // Select the namespace and database to use
    db.use_ns("28_Nodes").use_db("trading_system").await.context("Failed to use namespace/database")?;

    //schema::apply_schema(&db).await.context("Failed to apply schema")?;


    Ok(db)
}