use crate::structs::YouTubeCredentials;

pub fn get_oauth_client(
    credentials: &YouTubeCredentials,
) -> Result<oauth2::basic::BasicClient, String> {
    let client = oauth2::basic::BasicClient::new(
        oauth2::ClientId::new(credentials.client_id.clone()),
        Some(oauth2::ClientSecret::new(
            credentials.client_secret.expose_secret().clone(),
        )),
        oauth2::AuthUrl::new(credentials.auth_url.clone())
            .map_err(|e| e.to_string())?,
        Some(
            oauth2::TokenUrl::new(credentials.token_url.clone())
                .map_err(|e| e.to_string())?,
        ),
    )
    .set_redirect_uri(
        oauth2::RedirectUrl::new(credentials.redirect_url.clone())
            .map_err(|e| e.to_string())?,
    );

    Ok(client)
}
