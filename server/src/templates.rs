pub struct EmailContent {
    pub subject: String,
    pub html: String,
    pub text: String,
}

pub fn otp_email(code: &str) -> EmailContent {
    let subject = "Onyx Verification Code".to_string();

    // Split code into individual characters for digit-box styling
    let digits: String = code.chars().map(|c| {
        format!(
            r#"<td style="width: 48px; height: 60px; background-color: #141416; border-radius: 12px; text-align: center; vertical-align: middle; border: 1px solid rgba(255,255,255,0.06); font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; font-size: 30px; font-weight: 800; color: #fafafa; letter-spacing: 0;">{}</td>"#,
            c
        )
    }).collect::<Vec<_>>().join(r#"<td style="width: 8px;"></td>"#);

    let action_html = format!(
        r#"
        <tr>
          <td align="center" style="padding: 0 0 0 0;">
            <div style="max-height: 0; overflow: hidden; font-size: 1px; line-height: 1px; color: #030304; mso-hide: all;">{code}</div>
            <table cellpadding="0" cellspacing="0" border="0" style="border-collapse: collapse;" aria-hidden="true">
              <tr>
                {digits}
              </tr>
            </table>
          </td>
        </tr>
        <tr>
          <td align="center" style="padding: 12px 0 4px 0;">
            <p style="margin: 0; font-size: 12px; color: #52525b; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">Expires in <span style="color: #a78bfa; font-weight: 600;">15 minutes</span></p>
          </td>
        </tr>
        "#,
        code = code,
        digits = digits
    );

    let html = generate_base_template(
        "Verify Your Identity",
        "Enter this code in the app to continue.",
        &action_html,
    );

    let text = format!(
        "ONYX\n\nVerify your identity.\n\nYour code: {}\n\nExpires in 15 minutes.\n\nIf you didn't request this, ignore this email.\n\n—\nStateless relay · Zero-knowledge · No logs",
        code
    );

    EmailContent {
        subject,
        html,
        text,
    }
}

pub fn magic_link_email(link: &str) -> EmailContent {
    let subject = "Log in to Onyx".to_string();

    let action_html = format!(
        r#"
        <tr>
          <td align="center" style="padding: 4px 0 12px 0;">
            <table cellpadding="0" cellspacing="0" border="0" style="border-collapse: collapse;">
              <tr>
                <td align="center" style="background: linear-gradient(135deg, #7c3aed, #a855f7); border-radius: 14px; mso-padding-alt: 16px 44px;">
                  <a href="{}" style="display: inline-block; padding: 16px 44px; color: #ffffff; text-decoration: none; font-weight: 700; font-size: 15px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; letter-spacing: 0.3px;">Open Onyx</a>
                </td>
              </tr>
            </table>
          </td>
        </tr>
        <tr>
          <td align="center" style="padding: 12px 0 4px 0;">
            <p style="margin: 0; font-size: 12px; color: #52525b; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">Link expires in <span style="color: #a78bfa; font-weight: 600;">1 hour</span></p>
          </td>
        </tr>
        "#,
        link
    );

    let html = generate_base_template(
        "Welcome Back",
        "Tap below to decrypt your workspace.",
        &action_html,
    );

    let text = format!(
        "ONYX\n\nWelcome back.\n\nOpen this link to log in:\n{}\n\nExpires in 1 hour.\n\n—\nStateless relay · Zero-knowledge · No logs",
        link
    );

    EmailContent {
        subject,
        html,
        text,
    }
}

fn generate_base_template(title: &str, message: &str, content: &str) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="color-scheme" content="dark">
  <meta name="supported-color-schemes" content="dark">
  <title>Onyx</title>
  <!--[if mso]>
  <style>
    table {{ border-collapse: collapse; }}
    .card {{ background-color: #0a0a0c !important; }}
  </style>
  <![endif]-->
</head>
<body style="margin: 0; padding: 0; background-color: #030304; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; -webkit-font-smoothing: antialiased; -webkit-text-size-adjust: 100%;">

  <!-- Outer wrapper -->
  <table width="100%" cellpadding="0" cellspacing="0" border="0" style="background-color: #030304;">
    <tr>
      <td align="center" style="padding: 48px 20px;">

        <!-- Card -->
        <table width="100%" cellpadding="0" cellspacing="0" border="0" style="max-width: 440px; border-collapse: collapse;">
          <tr>
            <td class="card" style="background-color: #0a0a0c; border-radius: 24px; padding: 0; border: 1px solid rgba(255,255,255,0.04); overflow: hidden;">

              <!-- Purple accent bar at top -->
              <table width="100%" cellpadding="0" cellspacing="0" border="0" style="border-collapse: collapse;">
                <tr>
                  <td style="height: 3px; background: linear-gradient(90deg, transparent 0%, #7c3aed 25%, #a855f7 50%, #7c3aed 75%, transparent 100%);"></td>
                </tr>
              </table>

              <!-- Inner content -->
              <table width="100%" cellpadding="0" cellspacing="0" border="0" style="border-collapse: collapse; padding: 0;">
                <tr>
                  <td style="padding: 36px 32px 28px 32px;">

                    <table width="100%" cellpadding="0" cellspacing="0" border="0" style="border-collapse: collapse;">

                      <!-- Logo -->
                      <tr>
                        <td align="center" style="padding: 0 0 24px 0;">
                          <span style="font-size: 20px; font-weight: 800; color: #ffffff; letter-spacing: 4px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">ONYX</span><span style="color: #a855f7; font-size: 20px; font-weight: 800;">.</span>
                        </td>
                      </tr>

                      <!-- Title -->
                      <tr>
                        <td align="center" style="padding: 0 0 6px 0;">
                          <h1 style="margin: 0; font-size: 22px; font-weight: 700; color: #fafafa; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; letter-spacing: -0.3px;">{}</h1>
                        </td>
                      </tr>

                      <!-- Message -->
                      <tr>
                        <td align="center" style="padding: 0 0 28px 0;">
                          <p style="margin: 0; font-size: 14px; color: #71717a; line-height: 1.6; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">{}</p>
                        </td>
                      </tr>

                      <!-- Dynamic Content -->
                      {}

                      <!-- Divider -->
                      <tr>
                        <td style="padding: 18px 20px 16px 20px;">
                          <div style="height: 2px; background: linear-gradient(90deg, transparent, rgba(168, 85, 247, 0.25), transparent); border-radius: 1px;"></div>
                        </td>
                      </tr>

                      <!-- ZK Badge -->
                      <tr>
                        <td align="center" style="padding: 0 0 10px 0;">
                          <table cellpadding="0" cellspacing="0" border="0" style="border-collapse: collapse;">
                            <tr>
                              <td style="background-color: rgba(124, 58, 237, 0.08); border-radius: 100px; padding: 7px 18px; border: 1px solid rgba(255,255,255,0.06);">
                                <span style="font-size: 10px; font-weight: 700; color: #a78bfa; letter-spacing: 1.2px; text-transform: uppercase; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">&#128274;&ensp;Zero-Knowledge</span>
                              </td>
                            </tr>
                          </table>
                        </td>
                      </tr>

                      <!-- Footer -->
                      <tr>
                        <td align="center" style="padding: 0 0 4px 0;">
                          <p style="margin: 0; font-size: 10px; color: #3f3f46; line-height: 1.8; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; letter-spacing: 0.3px;">Dispatched via stateless relay<br>No metadata or content logged</p>
                        </td>
                      </tr>

                    </table>

                  </td>
                </tr>
              </table>

            </td>
          </tr>
        </table>

        <!-- Outside-card footer -->
        <table width="100%" cellpadding="0" cellspacing="0" border="0" style="max-width: 440px; border-collapse: collapse;">
          <tr>
            <td align="center" style="padding: 20px 0 0 0;">
              <p style="margin: 0; font-size: 10px; color: #27272a; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; letter-spacing: 0.3px;">If you didn't request this, you can safely ignore it.</p>
            </td>
          </tr>
        </table>

      </td>
    </tr>
  </table>
</body>
</html>
        "#,
        title, message, content
    )
}
