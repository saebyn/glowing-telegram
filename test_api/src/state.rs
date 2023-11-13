#[derive(Clone)]
pub struct AppState {
    openai_key: String,
    pool: crate::db::Pool,
}

impl AppState {
    pub fn new(openai_key: String, pool: crate::db::Pool) -> Self {
        Self { openai_key, pool }
    }

    pub fn openai_key(&self) -> String {
        self.openai_key.to_string()
    }

    pub fn pool(&self) -> &crate::db::Pool {
        &self.pool
    }
}
