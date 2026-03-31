use crate::mcp_server::security::McpError;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub url: String,
}

pub fn search_web(query: &str) -> Result<Vec<SearchResult>, McpError> {
    let enc = url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>();
    let url = format!("https://duckduckgo.com/html/?q={enc}");
    let resp = reqwest::blocking::get(url).map_err(|e| McpError::DaemonError(e.to_string()))?;
    let body = resp
        .text()
        .map_err(|e| McpError::DaemonError(format!("search response decode failed: {e}")))?;
    Ok(parse_html_results(&body))
}

pub fn parse_html_results(html: &str) -> Vec<SearchResult> {
    let link_re = Regex::new(
        r#"<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#,
    )
    .expect("valid link regex");
    let snippet_re =
        Regex::new(r#"<a[^>]*class="[^"]*result__snippet[^"]*"[^>]*>(.*?)</a>"#).expect("snippet");
    let tag_re = Regex::new(r"<[^>]+>").expect("tag");
    let mut snippets = snippet_re
        .captures_iter(html)
        .filter_map(|c| c.get(1).map(|m| tag_re.replace_all(m.as_str(), "").to_string()))
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    for cap in link_re.captures_iter(html).take(5) {
        let url = cap
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        let title = cap
            .get(2)
            .map(|m| tag_re.replace_all(m.as_str(), "").to_string())
            .unwrap_or_default();
        let snippet = if snippets.is_empty() {
            String::new()
        } else {
            snippets.remove(0)
        };
        out.push(SearchResult {
            title: html_unescape(&title),
            snippet: html_unescape(&snippet),
            url,
        });
    }
    out
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_html_response_into_results() {
        let html = r#"
            <a class="result__a" href="https://example.com/devops">DevOps Engineer Role</a>
            <a class="result__snippet">Own CI/CD and platform reliability.</a>
            <a class="result__a" href="https://example.com/sre">SRE Best Practices</a>
            <a class="result__snippet">Define SLOs and incident response.</a>
        "#;
        let parsed = super::parse_html_results(html);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].title, "DevOps Engineer Role");
        assert_eq!(parsed[0].url, "https://example.com/devops");
        assert!(parsed[0].snippet.contains("CI/CD"));
    }
}
