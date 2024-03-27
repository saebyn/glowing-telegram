use axum::extract::State;
use axum::Json;
use axum::{http::StatusCode, response::IntoResponse, routing::get};
use common_api_lib;
use dotenvy;
use redis;
use reqwest;
use serde_json::json;
use tracing::instrument;

#[derive(Clone, Debug)]
struct AppState {
    redis: redis::Client,
    twitch_client_id: String,
    twitch_client_secret: String,

    redirect_url: String,

    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let twitch_client_id = dotenvy::var("TWITCH_CLIENT_ID").expect("TWITCH_CLIENT_ID not set");
    let twitch_client_secret_path =
        dotenvy::var("TWITCH_CLIENT_SECRET_PATH").expect("TWITCH_CLIENT_SECRET_PATH not set");

    let state = AppState {
        redis: redis::Client::open(dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"))?,
        redirect_url: dotenvy::var("REDIRECT_URL").expect("REDIRECT_URL must be set"),

        http_client: reqwest::Client::builder()
            .user_agent("saebyn-twitch-api/0.1")
            .connection_verbose(true)
            .build()
            .expect("failed to create http client"),

        twitch_client_id,

        twitch_client_secret: std::fs::read_to_string(twitch_client_secret_path)
            .expect("failed to read twitch secret from TWITCH_CLIENT_SECRET_PATH")
            .trim()
            .to_string(),
    };

    common_api_lib::run(state, |app| {
        app.route("/login", get(get_login_handler).post(post_login_handler))
    })
    .await
}

#[instrument]
async fn get_login_handler(State(state): State<AppState>) -> impl IntoResponse {
    let scopes = vec!["chat:read"];

    let url = format!(
        "https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}",
        state.twitch_client_id,
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
        expires_in,
    } = get_token(&state, code).await;

    let mut con = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .expect("failed to get redis connection");

    let _: () = redis::AsyncCommands::set(&mut con, "twitch:refresh_token", refresh_token)
        .await
        .expect("failed to set refresh token");

    let opts =
        redis::SetOptions::default().with_expiration(redis::SetExpiry::EX(expires_in as usize));
    let _: () =
        redis::AsyncCommands::set_options(&mut con, "twitch:access_token", access_token, opts)
            .await
            .expect("failed to set access token");

    (StatusCode::ACCEPTED,)
}

struct AuthTokens {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

#[instrument]
async fn get_token(state: &AppState, code: &str) -> AuthTokens {
    let url = "https://id.twitch.tv/oauth2/token";

    // urlencoded form data
    let body = json!({
      "client_id": state.twitch_client_id,
      "client_secret": state.twitch_client_secret,
      "code": code,
      "grant_type": "authorization_code",
      "redirect_uri": state.redirect_url,
    });

    let response = state
        .http_client
        .post(url)
        .form(&body)
        .send()
        .await
        .expect("failed to send request")
        .json::<serde_json::Value>()
        .await
        .expect("failed to parse response");

    AuthTokens {
        access_token: response["access_token"]
            .as_str()
            .expect("access_token not found")
            .to_string(),
        refresh_token: response["refresh_token"]
            .as_str()
            .expect("refresh_token not found")
            .to_string(),
        expires_in: response["expires_in"]
            .as_u64()
            .expect("expires_in not found"),
    }
}
