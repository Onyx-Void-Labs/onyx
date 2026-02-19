use dotenv::dotenv;
use serde::Serialize;
use std::env;
use tauri::command;

#[derive(Serialize)]
struct LinkPayload {
    email: String,
    link: String,
}

#[derive(Serialize)]
struct OtpPayload {
    email: String,
    code: String,
}

#[command]
pub async fn send_magic_link_email(email: String, link: String) -> Result<String, String> {
    dotenv().ok(); // Load .env

    // Debug: Print env var status
    match env::var("RELAY_URL") {
        Ok(val) => println!("[DEBUG] Found RELAY_URL: {}", val),
        Err(_) => println!("[DEBUG] RELAY_URL not found in env! Falling back to SMTP."),
    }

    // Debug: Print env var status and CWD
    match env::current_dir() {
        Ok(path) => println!("[DEBUG] CWD: {}", path.display()),
        Err(e) => println!("[DEBUG] Could not get CWD: {}", e),
    }

    let relay_url = env::var("RELAY_URL").unwrap_or_else(|_| {
        println!("[DEBUG] RELAY_URL not set. Defaulting to http://127.0.0.1:3005");
        "http://127.0.0.1:3005".to_string()
    });

    // Check if we should use Relay (Assume yes if we have a URL, even default)
    // Note: To disable relay, one would need to explicitly NOT have this default logic
    // or use a flag. For this "Pure ZK" build, we prefer Relay.

    println!("Using Email Relay: {}", relay_url);
    let client = reqwest::Client::new();
    let payload = LinkPayload {
        email: email.clone(),
        link: link.clone(),
    };

    // Ensure relay_url has no trailing slash
    let base_url = relay_url.trim_end_matches('/');
    let url = format!("{}/api/email/link", base_url);

    let res = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        return Ok("Sent via Relay".to_string());
    } else {
        println!("Relay Error: {:?}", res.status());
        // Fallback to SMTP or fail? Ideally fail if strict.
        // But for dev, we might fall through.
        // Let's return error to force fix.
        return Err(format!("Relay failed: {}", res.status()));
    }

    /*
    // FALLBACK REMOVED: In ZK mode we only want Relay.
    // Direct SMTP (Dev Mode) - reachable only if Relay code above is commented out or logic changed
     */
}

#[command]
pub async fn send_otp_email(email: String, code: String) -> Result<String, String> {
    dotenv().ok(); // Load .env

    let relay_url = env::var("RELAY_URL").unwrap_or("http://127.0.0.1:3005".to_string());

    println!("Using Email Relay: {}", relay_url);
    let client = reqwest::Client::new();
    let payload = OtpPayload {
        email: email.clone(),
        code: code.clone(),
    };

    // Remove trailing slash if present on relay_url
    let base_url = relay_url.trim_end_matches('/');
    let url = format!("{}/api/email/otp", base_url);

    let res = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        return Ok("Sent via Relay".to_string());
    } else {
        return Err(format!("Relay failed: {}", res.status()));
    }
}

// Health Check Command
#[command]
pub async fn check_relay_health() -> Result<String, String> {
    dotenv().ok();

    let relay_url = env::var("RELAY_URL").unwrap_or("http://127.0.0.1:3005".to_string());
    println!("Checking Relay Health at: {}", relay_url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2)) // Fast timeout for check
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get(&relay_url)
        .send()
        .await
        .map_err(|e| format!("Relay Unreachable: {}", e))?;

    if res.status().is_success() {
        let text = res.text().await.map_err(|e| e.to_string())?;
        Ok(format!("Relay OK: {}", text))
    } else {
        Err(format!("Relay Error Status: {}", res.status()))
    }
}
