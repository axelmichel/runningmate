//! OAuth 1.0a request signing (HMAC-SHA1, RFC 5849), as `requests_oauthlib` does it for garth.

use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use url::Url;

/// Percent-encoding of RFC 3986: everything except unreserved characters.
pub fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub struct Credentials<'a> {
    pub consumer_key: &'a str,
    pub consumer_secret: &'a str,
    /// Access token and secret, once known.
    pub token: Option<(&'a str, &'a str)>,
}

/// `Authorization` header for a request. `body` holds form-encoded body parameters, which are
/// part of the signature.
pub fn authorization(
    method: &str,
    url: &Url,
    body: &[(String, String)],
    credentials: &Credentials,
    nonce: &str,
    timestamp: u64,
) -> String {
    let mut oauth = vec![
        (
            "oauth_consumer_key".to_string(),
            credentials.consumer_key.to_string(),
        ),
        ("oauth_nonce".to_string(), nonce.to_string()),
        (
            "oauth_signature_method".to_string(),
            "HMAC-SHA1".to_string(),
        ),
        ("oauth_timestamp".to_string(), timestamp.to_string()),
        ("oauth_version".to_string(), "1.0".to_string()),
    ];
    if let Some((token, _)) = credentials.token {
        oauth.push(("oauth_token".to_string(), token.to_string()));
    }

    let mut params: Vec<(String, String)> = oauth
        .iter()
        .cloned()
        .chain(
            url.query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned())),
        )
        .chain(body.iter().cloned())
        .map(|(k, v)| (encode(&k), encode(&v)))
        .collect();
    params.sort();
    let normalized = params
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");

    let mut base_url = url.clone();
    base_url.set_query(None);
    base_url.set_fragment(None);
    let base = format!(
        "{}&{}&{}",
        method.to_ascii_uppercase(),
        encode(base_url.as_str()),
        encode(&normalized)
    );
    let key = format!(
        "{}&{}",
        encode(credentials.consumer_secret),
        encode(credentials.token.map_or("", |(_, secret)| secret))
    );
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(key.as_bytes()).expect("HMAC takes any key");
    mac.update(base.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    oauth.push(("oauth_signature".to_string(), signature));
    let fields = oauth
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("OAuth {fields}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The worked example from Twitter's "Creating a signature" documentation.
    #[test]
    fn matches_the_published_example() {
        let url =
            Url::parse("https://api.twitter.com/1.1/statuses/update.json?include_entities=true")
                .unwrap();
        let body = [(
            "status".to_string(),
            "Hello Ladies + Gentlemen, a signed OAuth request!".to_string(),
        )];
        let header = authorization(
            "POST",
            &url,
            &body,
            &Credentials {
                consumer_key: "xvz1evFS4wEEPTGEFPHBog",
                consumer_secret: "kAcSOqF21Fu85e7zjz7ZN2U4ZRhfV3WpwPAoE3Z7kBw",
                token: Some((
                    "370773112-GmHxMAgYyLbNEtIKZeRNFsMKPR9EyMZeS9weJAEb",
                    "LswwdoUaIvS8ltyTt5jkRh4J50vUPVVHtR2YPi5kE",
                )),
            },
            "kYjzVBB8Y0ZFabxSWbWovY3uYSQ2pTgmZeNu2VS4cg",
            1318622958,
        );
        assert!(
            header.contains(r#"oauth_signature="hCtSmYh%2BiHYCEqBWrE7C7hYmtUk%3D""#),
            "{header}"
        );
    }

    #[test]
    fn encodes_reserved_characters() {
        assert_eq!(encode("a b+c/~"), "a%20b%2Bc%2F~");
    }
}
