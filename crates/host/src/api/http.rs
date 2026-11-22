//! `fetch`: HTTP requests on behalf of the app.
//!
//! A response is handed over in two steps, as in the Fetch API: `fetch`
//! resolves with the status and headers, and the body stays here until
//! JavaScript asks for it as text or as bytes. That way a body crosses the
//! boundary once, in the form it is wanted.

use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use blitz_traits::net::Url;
use serde_json::{Value, json};

use super::{Api, Scope, denied};

/// Enough for any API response; a download of this size should not go
/// through JSON and base64 anyway.
const MAX_BODY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;
/// Unread bodies kept at once. Old ones are dropped first.
const MAX_HELD_BODIES: usize = 32;

fn default_port(scheme: &str) -> Option<u16> {
    match scheme {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    }
}

/// Does one `permissions.http` entry cover this URL? An entry is an origin
/// with an optional path prefix; the host may start with `*.`.
fn pattern_matches(pattern: &str, url: &Url) -> bool {
    let Some((scheme, rest)) = pattern.split_once("://") else {
        return false;
    };
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, format!("/{path}")),
        None => (rest, "/".to_owned()),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if port.parse::<u16>().is_ok() => (host, port.parse().ok()),
        _ => (authority, default_port(scheme)),
    };
    let Some(actual) = url.host_str() else {
        return false;
    };
    let host_matches = match host.strip_prefix("*.") {
        // `*.example.com` covers `example.com` and anything under it, and
        // nothing that merely ends in the same letters.
        Some(suffix) => {
            actual.eq_ignore_ascii_case(suffix)
                || actual
                    .to_ascii_lowercase()
                    .ends_with(&format!(".{}", suffix.to_ascii_lowercase()))
        }
        None => actual.eq_ignore_ascii_case(host),
    };
    scheme == url.scheme()
        && host_matches
        && url.port_or_known_default() == port
        && url.path().starts_with(&path)
}

/// Whether the manifest lets the app talk to this URL.
pub fn url_allowed(scope: &Scope, url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    match scope {
        Scope::None => false,
        Scope::All => true,
        Scope::Some(patterns) => patterns.iter().any(|pattern| pattern_matches(pattern, url)),
    }
}

fn check(api: &Api, url: &Url) -> Result<(), String> {
    if url_allowed(&api.permissions.http, url) {
        Ok(())
    } else {
        let origin = url.origin().ascii_serialization();
        Err(denied(&format!("a request to {origin}"), "http"))
    }
}

pub(super) fn call(api: &Api, action: &str, args: &Value) -> Result<String, String> {
    match action {
        "fetch" => fetch(api, args),
        "body" => body(api, args),
        _ => Err(format!("this host has no call named http.{action}")),
    }
}

fn fetch(api: &Api, args: &Value) -> Result<String, String> {
    let requested = args["url"].as_str().ok_or("fetch needs a URL")?;
    let mut url = Url::parse(requested).map_err(|e| format!("{requested}: {e}"))?;
    let mut method = args["method"]
        .as_str()
        .unwrap_or("GET")
        .to_ascii_uppercase();
    let mut body = match (&args["body"], &args["body_base64"]) {
        (Value::String(text), _) => text.clone().into_bytes(),
        (_, Value::String(encoded)) => BASE64
            .decode(encoded)
            .map_err(|_| "the request body is not valid base64")?,
        _ => Vec::new(),
    };
    let headers: Vec<(String, String)> = args["headers"]
        .as_array()
        .map(|pairs| {
            pairs
                .iter()
                .filter_map(|pair| {
                    Some((pair[0].as_str()?.to_owned(), pair[1].as_str()?.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default();

    // Redirects are followed here, not by the client, so that every hop is
    // checked: an allowed server must not be able to send the app elsewhere.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .max_redirects(0)
        .max_redirects_will_error(false)
        .http_status_as_error(false)
        .build()
        .into();

    let mut hops = 0;
    let mut response = loop {
        check(api, &url)?;
        let mut request = ureq::http::Request::builder()
            .method(method.as_str())
            .uri(url.as_str());
        for (name, value) in &headers {
            request = request.header(name, value);
        }
        let request = request
            .body(body.clone())
            .map_err(|e| format!("bad request: {e}"))?;
        let response = agent.run(request).map_err(|e| format!("{url}: {e}"))?;

        let status = response.status().as_u16();
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let (true, Some(location)) = (matches!(status, 301 | 302 | 303 | 307 | 308), location)
        else {
            break response;
        };
        hops += 1;
        if hops > MAX_REDIRECTS {
            return Err(format!("{requested}: too many redirects"));
        }
        url = url
            .join(&location)
            .map_err(|e| format!("bad redirect to {location}: {e}"))?;
        // As browsers do: these turn into a plain GET.
        if matches!(status, 301 | 302 | 303) && method != "HEAD" {
            method = "GET".into();
            body.clear();
        }
    };
    let redirected = hops > 0;

    let status = response.status();
    let header_list: Vec<[String; 2]> = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            Some([name.as_str().to_owned(), value.to_str().ok()?.to_owned()])
        })
        .collect();
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_BYTES)
        .read_to_vec()
        .map_err(|e| format!("cannot read the response from {url}: {e}"))?;

    let id = {
        let mut next = api.next_body.lock().unwrap();
        *next += 1;
        *next
    };
    let mut bodies = api.bodies.lock().unwrap();
    if bodies.len() >= MAX_HELD_BODIES
        && let Some(oldest) = bodies.keys().min().copied()
    {
        bodies.remove(&oldest);
    }
    bodies.insert(id, bytes);

    Ok(json!({
        "id": id,
        "status": status.as_u16(),
        "status_text": status.canonical_reason().unwrap_or(""),
        "url": url.as_str(),
        "redirected": redirected,
        "headers": header_list,
    })
    .to_string())
}

/// Hand over a response body, once, as text or as base64.
fn body(api: &Api, args: &Value) -> Result<String, String> {
    let id = args["id"].as_u64().ok_or("http.body needs an id")?;
    let bytes = api
        .bodies
        .lock()
        .unwrap()
        .remove(&id)
        .ok_or("the response body was already read, or is no longer kept")?;
    let value = match args["as"].as_str() {
        Some("base64") => BASE64.encode(bytes),
        _ => String::from_utf8_lossy(&bytes).into_owned(),
    };
    Ok(Value::String(value).to_string())
}
