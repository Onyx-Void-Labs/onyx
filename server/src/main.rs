use axum::{
    extract::Json,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod templates; // Import the templates module

#[tokio::main]
async fn main() {
    dotenv().ok();
    
    // Initialize logging (for server health, NOT for user data)
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // CORS: Allow requests from anywhere (since Client is Tauri/App)
    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/", get(health_check))
        .route("/api/email/otp", post(send_otp))
        .route("/api/email/link", post(send_magic_link))
        .layer(cors);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = SocketAddr::from(([0, 0, 0, 0], port.parse().unwrap()));

    tracing::info!("Onyx Relay listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "Onyx Relay OK"
}

// --- OTP Handler ---

#[derive(Deserialize)]
struct OtpRequest {
    email: String,
    code: String,
}

async fn send_otp(Json(payload): Json<OtpRequest>) -> Result<String, String> {
    tracing::info!("Received OTP request. Spawning background task...");

    // Generate Content via Templates
    let content = templates::otp_email(&payload.code);

    // Spawn logging/sending in background so Client doesn't wait/timeout
    tokio::spawn(async move {
        match send_email(payload.email.clone(), &content.subject, content.html, content.text).await {
            Ok(_) => tracing::info!("OTP sent successfully to {}", payload.email),
            Err(e) => tracing::error!("Failed to send OTP to {}: {}", payload.email, e),
        }
    });

    Ok("Queued".to_string())
}

// --- Magic Link Handler ---

#[derive(Deserialize)]
struct LinkRequest {
    email: String,
    link: String,
}

async fn send_magic_link(Json(payload): Json<LinkRequest>) -> Result<String, String> {
    tracing::info!("Received Magic Link request. Spawning background task...");

    let content = templates::magic_link_email(&payload.link);

    tokio::spawn(async move {
        match send_email(payload.email.clone(), &content.subject, content.html, content.text).await {
            Ok(_) => tracing::info!("Magic Link sent successfully to {}", payload.email),
            Err(e) => tracing::error!("Failed to send Magic Link to {}: {}", payload.email, e),
        }
    });

    Ok("Queued".to_string())
}

// --- SMTP Logic ---

// --- SMTP Logic ---

async fn send_email(to: String, subject: &str, html_body: String, text_body: String) -> Result<(), String> {
    let host = env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.purelymail.com".to_string());
    let username = env::var("SMTP_USERNAME").map_err(|_| "SMTP_USERNAME not set")?;
    let password = env::var("SMTP_PASSWORD").map_err(|_| "SMTP_PASSWORD not set")?;

    let from_header = format!("Onyx <{}>", username);

    // Build Multipart Email (Text + HTML)
    // This is crucial for spam scores.
    let email = Message::builder()
        .from(from_header.parse::<Mailbox>().unwrap())
        .to(to.parse::<Mailbox>().map_err(|e| e.to_string())?)
        .subject(subject)
        .multipart(
            lettre::message::MultiPart::alternative() // "Alternative" means client chooses best view (HTML or Text)
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(lettre::message::header::ContentType::TEXT_PLAIN)
                        .body(text_body)
                )
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(lettre::message::header::ContentType::TEXT_HTML)
                        .body(html_body)
                )
        )
        .map_err(|e| e.to_string())?;

    let creds = Credentials::new(username, password);

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
            .map_err(|e| e.to_string())?
            .credentials(creds)
            .build();

    mailer.send(email).await.map_err(|e| e.to_string())?;

    Ok(())
}
