#[derive(Clone, Debug)]
pub struct AppState {
    pub pool: common_api_lib::db::Pool,
}

impl AppState {
    pub fn new(pool: common_api_lib::db::Pool) -> Self {
        Self { pool: pool }
    }

    pub fn pool(&self) -> &common_api_lib::db::Pool {
        &self.pool
    }
}
