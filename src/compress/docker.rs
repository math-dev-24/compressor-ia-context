use super::Compressor;
use super::truncate::{dedup_lines, truncate};

/// Pure compressor for docker / kubectl output.
pub struct DockerCompressor;

impl Compressor for DockerCompressor {
    fn compress(&self, raw: &str, sub: Option<&str>) -> String {
        match sub.unwrap_or("") {
            "ps" => compress_ps(raw),
            "images" => compress_images(raw),
            "logs" => dedup_lines(raw),
            _ => truncate(raw),
        }
    }
}

/// Compress `docker ps` — keep header + compact rows.
fn compress_ps(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return "[docker ps] no containers".into();
    }

    let mut out = format!("[containers: {}]\n", lines.len().saturating_sub(1));
    // Keep header
    if let Some(header) = lines.first() {
        out.push_str(&format!("{header}\n"));
    }
    for line in lines.iter().skip(1).take(30) {
        out.push_str(&format!("{line}\n"));
    }
    if lines.len() > 31 {
        out.push_str(&format!("  … +{} more\n", lines.len() - 31));
    }
    out
}

/// Compress `docker images` — similar approach.
fn compress_images(raw: &str) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return "[docker images] none".into();
    }

    let mut out = format!("[images: {}]\n", lines.len().saturating_sub(1));
    if let Some(header) = lines.first() {
        out.push_str(&format!("{header}\n"));
    }
    for line in lines.iter().skip(1).take(30) {
        out.push_str(&format!("{line}\n"));
    }
    if lines.len() > 31 {
        out.push_str(&format!("  … +{} more\n", lines.len() - 31));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress::Compressor;

    // ── compress_ps ──

    #[test]
    fn test_ps_empty() {
        let result = compress_ps("");
        assert_eq!(result, "[docker ps] no containers");
    }

    #[test]
    fn test_ps_with_containers() {
        let raw = "\
CONTAINER ID   IMAGE     COMMAND   STATUS
abc123         nginx     nginx     Up 2 hours
def456         redis     redis     Up 5 min
";
        let result = compress_ps(raw);
        assert!(result.contains("[containers: 2]"));
        assert!(result.contains("CONTAINER ID"));
        assert!(result.contains("abc123"));
        assert!(result.contains("def456"));
    }

    #[test]
    fn test_ps_header_only() {
        let raw = "CONTAINER ID   IMAGE     COMMAND   STATUS\n";
        let result = compress_ps(raw);
        assert!(result.contains("[containers: 0]"));
        assert!(result.contains("CONTAINER ID"));
    }

    #[test]
    fn test_ps_many_containers_truncated() {
        let mut raw = String::from("HEADER\n");
        for i in 0..35 {
            raw.push_str(&format!("container_{i}  image  cmd  Up\n"));
        }
        let result = compress_ps(&raw);
        // 36 lines total (1 header + 35 data), show header + 30 data = 31
        assert!(result.contains("[containers: 35]"));
        assert!(result.contains("… +5 more"));
        assert!(result.contains("container_0"));
        assert!(result.contains("container_29"));
        assert!(!result.contains("container_34"));
    }

    // ── compress_images ──

    #[test]
    fn test_images_empty() {
        let result = compress_images("");
        assert_eq!(result, "[docker images] none");
    }

    #[test]
    fn test_images_normal() {
        let raw = "\
REPOSITORY   TAG       IMAGE ID       SIZE
nginx        latest    abc123         150MB
redis        7.0       def456         120MB
";
        let result = compress_images(raw);
        assert!(result.contains("[images: 2]"));
        assert!(result.contains("REPOSITORY"));
        assert!(result.contains("nginx"));
        assert!(result.contains("redis"));
    }

    // ── logs dedup ──

    #[test]
    fn test_logs_dedup() {
        let c = DockerCompressor;
        let raw = "\
[INFO] Starting server
[INFO] Request handled
[INFO] Request handled
[INFO] Request handled
[INFO] Shutting down
";
        let result = c.compress(raw, Some("logs"));
        assert!(result.contains("Request handled  (×3)"));
        assert!(result.contains("Starting server"));
        assert!(result.contains("Shutting down"));
    }

    // ── Compressor trait dispatch ──

    #[test]
    fn test_trait_dispatches_ps() {
        let c = DockerCompressor;
        let result = c.compress("", Some("ps"));
        assert!(result.contains("[docker ps] no containers"));
    }

    #[test]
    fn test_trait_dispatches_images() {
        let c = DockerCompressor;
        let result = c.compress("", Some("images"));
        assert!(result.contains("[docker images] none"));
    }

    #[test]
    fn test_trait_fallback() {
        let c = DockerCompressor;
        let raw = "some docker output";
        let result = c.compress(raw, Some("inspect"));
        assert!(result.contains("some docker output"));
    }

    #[test]
    fn test_trait_none_sub() {
        let c = DockerCompressor;
        let result = c.compress("fallback", None);
        assert!(result.contains("fallback"));
    }
}
