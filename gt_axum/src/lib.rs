pub mod cognito;

use axum::Router;

/// Runs an Axum router in either local development mode or Lambda runtime mode.
///
/// In debug builds (`#[cfg(debug_assertions)]`), the router is served on
/// `127.0.0.1:3030` using a standard Axum server.
///
/// In release builds, the router is wrapped with the Lambda runtime adapter
/// and executed as a Lambda function.
///
/// # Errors
///
/// Returns an error if:
/// - The local server fails to bind to the port (debug mode)
/// - The Lambda runtime fails to start (release mode)
///
/// # Panics
///
/// Panics if the Lambda runtime fails to start (release mode only).
pub async fn run_app(app: Router) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3030));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("Starting local development server on http://127.0.0.1:3030");
        axum::serve(listener, app).await?;
    }

    #[cfg(not(debug_assertions))]
    {
        use lambda_http::tower;

        let app = tower::ServiceBuilder::new()
            .layer(axum_aws_lambda::LambdaLayer::default().trim_stage())
            .service(app);

        lambda_http::run(app).await.unwrap();
    }

    Ok(())
}
