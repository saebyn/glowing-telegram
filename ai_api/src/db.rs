use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use dotenvy::dotenv;
use std::env;

pub type Pool = bb8::Pool<AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>>;

/**
 * Establishes a connection to the database.
 *
 * Returns a bb8::Pool connection pool.
 */
pub async fn create_pool() -> Pool {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(database_url);
    bb8::Pool::builder()
        .test_on_check_out(true)
        .max_size(10)
        .build(config)
        .await
        .unwrap()
}
