#[derive(Clone)]
pub struct AppState {
    openai_key: String,
}

impl AppState {
    pub fn new(openai_key: String) -> Self {
        Self { openai_key }
    }

    pub fn openai_key(&self) -> String {
        self.openai_key.to_string()
    }
}
