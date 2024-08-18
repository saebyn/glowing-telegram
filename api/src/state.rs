use crate::config;

#[derive(Clone, Debug)]
pub struct AppState {
    pub config: config::Config,

    pub http_client: reqwest::Client,
    pub redis_client: redis::Client,
}

impl AppState {
    pub fn new(config: config::Config) -> Self {
        let redis_url = config.redis_url.clone();
        let http_client_agent = config.http_client_agent.clone();

        Self {
            config,

            redis_client: redis::Client::open(redis_url)
                .expect("failed to open redis client"),

            http_client: reqwest::Client::builder()
                .user_agent(http_client_agent)
                .connection_verbose(false)
                .build()
                .expect("failed to create http client"),
        }
    }
}
