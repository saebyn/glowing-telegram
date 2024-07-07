use redact::Secret;

#[derive(Clone, Debug)]
pub struct AppState {
    pub openai_key: Secret<String>,
    pub openai_model: String,

    pub video_storage_path: String,
    pub rendered_episode_storage_path: String,
    pub noise: f64,
    pub duration: f64,

    pub task_api_url: String,
    pub task_api_external_url: String,

    pub this_api_base_url: String,

    pub twitch_client_id: String,
    pub twitch_client_secret: Secret<String>,
    pub twitch_user_id: String,
    pub twitch_redirect_url: String,

    pub youtube_auth_uri: String,
    pub youtube_token_uri: String,
    pub youtube_client_id: String,
    pub youtube_client_secret: Secret<String>,
    pub youtube_redirect_url: String,

    pub http_client: reqwest::Client,
    pub redis: redis::Client,
}

impl AppState {
    pub fn new(openai_key_path: String, openai_model: String) -> Self {
        let twitch_client_id = dotenvy::var("TWITCH_CLIENT_ID").expect("TWITCH_CLIENT_ID not set");
    let twitch_client_secret_path =
        dotenvy::var("TWITCH_CLIENT_SECRET_PATH").expect("TWITCH_CLIENT_SECRET_PATH not set");
    let twitch_user_id = dotenvy::var("TWITCH_USER_ID").expect("TWITCH_USER_ID not set");

    let youtube_auth_uri = dotenvy::var("YOUTUBE_AUTH_URI").expect("YOUTUBE_AUTH_URI not set");
    let youtube_token_uri = dotenvy::var("YOUTUBE_TOKEN_URI").expect("YOUTUBE_TOKEN_URI not set");
    let youtube_client_id = dotenvy::var("YOUTUBE_CLIENT_ID").expect("YOUTUBE_CLIENT_ID not set");
    let youtube_client_secret_path =
        dotenvy::var("YOUTUBE_CLIENT_SECRET_PATH").expect("YOUTUBE_CLIENT_SECRET_PATH not set");

        Self {
            openai_key: Secret::new(
                std::fs::read_to_string(openai_key_path)
                    .expect("failed to read openai key from OPENAI_KEY_PATH")
                    .trim()
                    .to_string(),
            ),
            openai_model,

            video_storage_path: dotenvy::var("VIDEO_STORAGE_PATH")
                .expect("VIDEO_STORAGE_PATH must be set"),

            rendered_episode_storage_path: dotenvy::var(
                "RENDERED_EPISODE_STORAGE_PATH",
            )
            .expect("RENDERED_EPISODE_STORAGE_PATH must be set"),

            noise: dotenvy::var("NOISE")
                .expect("NOISE must be set")
                .parse::<f64>()
                .expect("NOISE must be a float"),

            duration: dotenvy::var("DURATION")
                .expect("DURATION must be set")
                .parse::<f64>()
                .expect("DURATION must be a float"),

            task_api_url: dotenvy::var("TASK_API_URL")
                .expect("TASK_API_URL must be set"),

            task_api_external_url: dotenvy::var("TASK_API_EXTERNAL_URL")
                .expect("TASK_API_EXTERNAL_URL must be set"),

            this_api_base_url: dotenvy::var("THIS_API_BASE_URL")
                .expect("THIS_API_BASE_URL must be set"),

            redis: redis::Client::open(
                dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"),
            )
            .expect("failed to open redis client"),


            youtube_redirect_url: dotenvy::var("YOUTUBE_REDIRECT_URL").expect("YOUTUBE_REDIRECT_URL must be set"),
            twitch_redirect_url: dotenvy::var("TWITCH_REDIRECT_URL").expect("TWITCH_REDIRECT_URL must be set"),

            http_client: reqwest::Client::builder()
                .user_agent("saebyn-api/0.1")
                .connection_verbose(false)
                .build()
                .expect("failed to create http client"),
    
            twitch_client_id,
            twitch_user_id,
    
            twitch_client_secret: Secret::new(
                std::fs::read_to_string(twitch_client_secret_path)
                    .expect("failed to read twitch secret from TWITCH_CLIENT_SECRET_PATH")
                    .trim()
                    .to_string(),
            ),

            
        youtube_auth_uri,
        youtube_token_uri,
        youtube_client_id,
        youtube_client_secret: Secret::new(
            std::fs::read_to_string(youtube_client_secret_path)
                .expect("failed to read youtube secret from YOUTUBE_CLIENT_SECRET_PATH")
                .trim()
                .to_string()    ),
        }
    }
}
