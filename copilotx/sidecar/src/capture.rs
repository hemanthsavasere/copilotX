use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use xcap::Monitor;

pub fn capture_primary_monitor() -> Result<String> {
    let monitors = Monitor::all().context("Failed to enumerate monitors")?;
    let primary = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .context("No primary monitor found")?;

    let image = primary.capture_image().context("Failed to capture screen")?;

    let mut png_buf = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png_buf), image::ImageFormat::Png)
        .context("Failed to encode screenshot to PNG")?;

    let b64 = BASE64.encode(&png_buf);
    Ok(b64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_capture_primary_monitor_real() {
        let result = capture_primary_monitor();
        assert!(result.is_ok());
        let b64 = result.unwrap();
        assert!(!b64.is_empty());
        assert!(b64.starts_with("iVBOR"));
    }
}
