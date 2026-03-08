// ─── Onyx Core — Embed Parser (Media URL Detection) ────────────────

use regex::Regex;

/// Metadata about a recognized media embed.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaMeta {
    pub provider: String,
    pub id: String,
    pub thumb_url: Option<String>,
}

/// Parse a URL and return (provider, video/resource ID) if recognized.
/// Legacy tuple API — delegates to `parse_embed_meta`.
pub fn parse_embed(url: &str) -> Option<(String, String)> {
    parse_embed_meta(url).map(|m| (m.provider, m.id))
}

/// Parse a URL and return a `MediaMeta` with provider, ID, and optional thumbnail.
/// Recognizes YouTube, Vimeo, Twitter/X, PDF links, and common image URLs.
pub fn parse_embed_meta(url: &str) -> Option<MediaMeta> {
    // YouTube: https://www.youtube.com/watch?v=ID or https://youtu.be/ID
    let yt_long = match Regex::new(r"(?:youtube\.com/watch\?.*v=)([\w\-]{11})") {
        Ok(r) => r,
        Err(_) => return None,
    };
    let yt_short = match Regex::new(r"(?:youtu\.be/)([\w\-]{11})") {
        Ok(r) => r,
        Err(_) => return None,
    };

    if let Some(caps) = yt_long.captures(url) {
        let id = caps[1].to_string();
        return Some(MediaMeta {
            provider: "youtube".into(),
            thumb_url: Some(format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id)),
            id,
        });
    }
    if let Some(caps) = yt_short.captures(url) {
        let id = caps[1].to_string();
        return Some(MediaMeta {
            provider: "youtube".into(),
            thumb_url: Some(format!("https://img.youtube.com/vi/{}/hqdefault.jpg", id)),
            id,
        });
    }

    // Vimeo: https://vimeo.com/ID
    let vimeo = match Regex::new(r"vimeo\.com/(\d+)") {
        Ok(r) => r,
        Err(_) => return None,
    };
    if let Some(caps) = vimeo.captures(url) {
        return Some(MediaMeta {
            provider: "vimeo".into(),
            id: caps[1].to_string(),
            thumb_url: None,
        });
    }

    // Twitter/X: https://twitter.com/user/status/ID or https://x.com/user/status/ID
    let tweet = match Regex::new(r"(?:twitter\.com|x\.com)/\w+/status/(\d+)") {
        Ok(r) => r,
        Err(_) => return None,
    };
    if let Some(caps) = tweet.captures(url) {
        return Some(MediaMeta {
            provider: "tweet".into(),
            id: caps[1].to_string(),
            thumb_url: None,
        });
    }

    // PDF: URL ending in .pdf
    let pdf = match Regex::new(r"(?i)^https?://\S+\.pdf$") {
        Ok(r) => r,
        Err(_) => return None,
    };
    if pdf.is_match(url) {
        return Some(MediaMeta {
            provider: "pdf".into(),
            id: url.to_string(),
            thumb_url: None,
        });
    }

    // Image: URL ending in common image extensions
    let image = match Regex::new(r"(?i)^https?://\S+\.(png|jpe?g|gif|webp|svg|bmp)$") {
        Ok(r) => r,
        Err(_) => return None,
    };
    if image.is_match(url) {
        return Some(MediaMeta {
            provider: "image".into(),
            id: url.to_string(),
            thumb_url: Some(url.to_string()),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_long_url() {
        let result = parse_embed("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert_eq!(result, Some(("youtube".into(), "dQw4w9WgXcQ".into())));
    }

    #[test]
    fn youtube_short_url() {
        let result = parse_embed("https://youtu.be/dQw4w9WgXcQ");
        assert_eq!(result, Some(("youtube".into(), "dQw4w9WgXcQ".into())));
    }

    #[test]
    fn vimeo_url() {
        let result = parse_embed("https://vimeo.com/123456789");
        assert_eq!(result, Some(("vimeo".into(), "123456789".into())));
    }

    #[test]
    fn tweet_url() {
        let result = parse_embed("https://x.com/user/status/1234567890");
        assert_eq!(result, Some(("tweet".into(), "1234567890".into())));
    }

    #[test]
    fn unknown_url() {
        assert_eq!(parse_embed("https://example.com/page"), None);
    }

    #[test]
    fn pdf_url() {
        let meta = parse_embed_meta("https://example.com/paper.pdf");
        assert_eq!(
            meta,
            Some(MediaMeta {
                provider: "pdf".into(),
                id: "https://example.com/paper.pdf".into(),
                thumb_url: None,
            })
        );
    }

    #[test]
    fn image_url() {
        let meta = parse_embed_meta("https://example.com/photo.png");
        assert_eq!(
            meta,
            Some(MediaMeta {
                provider: "image".into(),
                id: "https://example.com/photo.png".into(),
                thumb_url: Some("https://example.com/photo.png".into()),
            })
        );
    }

    #[test]
    #[allow(clippy::panic)]
    fn youtube_meta_has_thumbnail() {
        if let Some(meta) = parse_embed_meta("https://youtu.be/dQw4w9WgXcQ") {
            assert_eq!(meta.provider, "youtube");
            assert!(meta.thumb_url.is_some());
        } else {
            panic!("failed to parse youtube meta");
        }
    }
}
