#[derive(Clone)]
pub struct AppState {
    openai_key: String,
    openai_model: String,
}

impl AppState {
    pub fn new(openai_key: String, openai_model: String) -> Self {
        Self {
            openai_key,
            openai_model,
        }
    }

    pub fn openai_key(&self) -> String {
        self.openai_key.to_string()
    }

    pub fn openai_model(&self) -> String {
        self.openai_model.to_string()
    }
}
