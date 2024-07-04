use redact::Secret;

#[derive(Clone)]
pub struct AppState {
    pub openai_key: Secret<String>,
    pub openai_model: String,
}

impl AppState {
    pub fn new(openai_key_path: String, openai_model: String) -> Self {
        Self {
            openai_key: Secret::new(
                std::fs::read_to_string(openai_key_path)
                    .expect("failed to read openai key from OPENAI_KEY_PATH")
                    .trim()
                    .to_string(),
            ),
            openai_model,
        }
    }
}
