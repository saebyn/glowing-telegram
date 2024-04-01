use axum::extract::{FromRequestParts, State};

use axum::http::request::Parts;
use axum::{async_trait, Json};
use axum::{http::StatusCode, response::IntoResponse, routing::get};
use common_api_lib;
use dotenvy;
use redis;
use reqwest;
use serde::Serialize;
use serde_json::json;
use tracing::instrument;

// Redis key constants
const ACCESS_TOKEN_KEY: &str = "youtube:access_token";
const REFRESH_TOKEN_KEY: &str = "youtube:refresh_token";

// The state of the application.
#[derive(Clone, Debug)]
struct AppState {
    redis: redis::Client,
    youtube_client_id: String,
    youtube_client_secret: String,

    redirect_url: String,

    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let youtube_client_id = dotenvy::var("YOUTUBE_CLIENT_ID").expect("YOUTUBE_CLIENT_ID not set");
    let youtube_client_secret_path =
        dotenvy::var("YOUTUBE_CLIENT_SECRET_PATH").expect("YOUTUBE_CLIENT_SECRET_PATH not set");

    let state = AppState {
        redis: redis::Client::open(dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"))?,
        redirect_url: dotenvy::var("REDIRECT_URL").expect("REDIRECT_URL must be set"),

        http_client: reqwest::Client::builder()
            .user_agent("saebyn-youtube-api/0.1")
            .connection_verbose(false)
            .build()
            .expect("failed to create http client"),

        youtube_client_id,

        youtube_client_secret: std::fs::read_to_string(youtube_client_secret_path)
            .expect("failed to read youtube secret from YOUTUBE_CLIENT_SECRET_PATH")
            .trim()
            .to_string(),
    };

    common_api_lib::run(state, |app| {
        app.route("/login", get(get_login_handler).post(post_login_handler))
    })
    .await
}

#[derive(Serialize, Debug)]
struct YoutubeUpload {
    title: String,
    description: String,
    tags: Vec<String>,
    category: String,
    render_uri: String,
    thumbnail_uri: Option<String>,
    notify_subscribers: bool,
}

#[derive(Serialize, Debug)]
struct AuthTokens {
    access_token: String,
    refresh_token: String,
}

/**
 * Extractor for the access token from Redis.
 *
 * This is a simple extractor that gets the access token from Redis
 * and injects it into the request's extensions.
 */
struct AccessToken(String);

#[async_trait]
impl FromRequestParts<AppState> for AccessToken {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let mut con = match state.redis.get_multiplexed_async_connection().await {
            Ok(con) => con,
            Err(_) => {
                tracing::error!("failed to get redis connection");

                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal server error" })),
                ));
            }
        };

        let access_token = redis::AsyncCommands::get(&mut con, ACCESS_TOKEN_KEY).await;

        match access_token {
            Ok(access_token) => Ok(AccessToken(access_token)),
            Err(_) => {
                tracing::error!("failed to get access token from Redis");

                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "unauthorized" })),
                ))
            }
        }
    }
}

#[instrument]
async fn get_login_handler(State(state): State<AppState>) -> impl IntoResponse {
    let scopes = vec!["chat:read"];

    let url = format!(
        "https://id.youtube.tv/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}",
        state.youtube_client_id,
        state.redirect_url,
        scopes.join("+")
    );

    (StatusCode::OK, Json(json!({ "url": url })))
}

/**
 * POST /login
 *
 * Completes the OAuth flow by exchanging the code for an access token
 * and refresh token. Stores the tokens in Redis and returns a
 * 202 Accepted response.
 */
#[instrument]
async fn post_login_handler(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let code = body["code"].as_str().expect("code not found in body");

    let AuthTokens {
        access_token,
        refresh_token,
    } = match get_token(&state, code).await {
        Ok(tokens) => tokens,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR,);
        }
    };

    let mut con = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .expect("failed to get redis connection");

    let _: () = redis::AsyncCommands::set(&mut con, REFRESH_TOKEN_KEY, refresh_token)
        .await
        .expect("failed to set refresh token");

    let _: () = redis::AsyncCommands::set(&mut con, ACCESS_TOKEN_KEY, access_token)
        .await
        .expect("failed to set access token");

    (StatusCode::ACCEPTED,)
}

/**
 * Updates the refresh token in Redis.
 *
 * This function is called when the access token is not found in Redis.
 * It uses the refresh token to get a new access token and refresh token
 * from the Youtube API.
 */
async fn update_refresh_token(state: &AppState) -> Result<AuthTokens, ()> {
    let mut con = match state.redis.get_multiplexed_async_connection().await {
        Ok(con) => con,
        Err(_) => {
            tracing::error!("failed to get redis connection");

            return Err(());
        }
    };

    let refresh_token: Result<String, _> =
        redis::AsyncCommands::get(&mut con, REFRESH_TOKEN_KEY).await;

    let refresh_token = match refresh_token {
        Ok(refresh_token) => refresh_token.to_string(),
        Err(_) => {
            tracing::error!("failed to get refresh token from Redis");

            return Err(());
        }
    };

    get_refresh_token(state, &refresh_token).await
}

/**
 * Gets the access token and refresh token from the Youtube API.
 *
 * This function is called when the access token is not found in Redis.
 * It uses the refresh token to get a new access token and refresh token
 * from the Youtube API.
 */
async fn get_token(state: &AppState, code: &str) -> Result<AuthTokens, ()> {
    let url = "https://id.youtube.tv/oauth2/token";

    let body = json!({
        "client_id": state.youtube_client_id,
        "client_secret": state.youtube_client_secret,
        "code": code,
        "grant_type": "authorization_code",
        "redirect_uri": state.redirect_url,
    });

    let response = match state.http_client.post(url).json(&body).send().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("failed to send request to Youtube API");

            return Err(());
        }
    };

    let response = match response.json::<serde_json::Value>().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("failed to parse response from Youtube API");

            return Err(());
        }
    };

    Ok(AuthTokens {
        access_token: response["access_token"]
            .as_str()
            .expect("access_token not found")
            .to_string(),
        refresh_token: response["refresh_token"]
            .as_str()
            .expect("refresh_token not found")
            .to_string(),
    })
}

/**
 * Gets the access token and refresh token from the Youtube API.
 *
 * This function is called when the access token is not found in Redis.
 * It uses the refresh token to get a new access token and refresh token
 * from the Youtube API.
 */
async fn get_refresh_token(state: &AppState, refresh_token: &str) -> Result<AuthTokens, ()> {
    let url = "https://id.youtube.tv/oauth2/token";

    let body = json!({
        "client_id": state.youtube_client_id,
        "client_secret": state.youtube_client_secret,
        "refresh_token": refresh_token,
        "grant_type": "refresh_token",
    });

    let response = match state.http_client.post(url).json(&body).send().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("failed to send request to Youtube API");

            return Err(());
        }
    };

    let response = match response.json::<serde_json::Value>().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("failed to parse response from Youtube API");

            return Err(());
        }
    };

    Ok(AuthTokens {
        access_token: response["access_token"]
            .as_str()
            .expect("access_token not found")
            .to_string(),
        refresh_token: response["refresh_token"]
            .as_str()
            .expect("refresh_token not found")
            .to_string(),
    })
}
