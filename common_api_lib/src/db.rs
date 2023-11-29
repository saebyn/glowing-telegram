use axum::RequestPartsExt;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    Extension,
};
use diesel_async::pooled_connection::{bb8::PooledConnection, AsyncDieselConnectionManager};
use dotenvy::dotenv;
use std::env;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;

pub type Pool = diesel_async::pooled_connection::bb8::Pool<diesel_async::AsyncPgConnection>;

/**
 * Establishes a connection to the database.
 *
 * Returns a bb8::Pool connection pool.
 */
pub async fn create_pool() -> Pool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(database_url);
    Pool::builder()
        .test_on_check_out(true)
        .max_size(10)
        .build(config)
        .await
        .unwrap()
}

pub struct ConnectionWrapper<'a> {
    pub connection: PooledConnection<'a, diesel_async::AsyncPgConnection>,
}

impl Debug for ConnectionWrapper<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("connection", &"PooledConnection")
            .finish()
    }
}

/**
 * Provide an extractor for the database connection by getting the extension from the request for the pool.
 */
pub struct DbConnection<'a>(pub ConnectionWrapper<'a>);

#[async_trait]
impl<'a, S> FromRequestParts<S> for DbConnection<'a>
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let pool = match parts.extract::<Extension<Pool>>().await {
            Ok(pool) => pool.0,
            Err(e) => {
                tracing::error!("Error getting pool from request: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error getting pool from request",
                ));
            }
        };

        match pool.get_owned().await {
            Ok(conn) => Ok(Self(ConnectionWrapper { connection: conn })),
            Err(e) => {
                tracing::error!("Error getting connection from pool: {}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error getting connection from pool",
                ))
            }
        }
    }
}
