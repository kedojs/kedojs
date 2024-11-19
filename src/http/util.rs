use hyper::Uri;

///  
/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Authentication
pub fn basic_auth(username: &str, password: Option<&str>) -> String {
    use base64::prelude::BASE64_STANDARD;
    use base64::write::EncoderWriter;
    use std::io::Write;

    let mut buf = b"Basic ".to_vec();
    {
        let mut encoder = EncoderWriter::new(&mut buf, &BASE64_STANDARD);
        let _ = write!(encoder, "{username}:");
        if let Some(password) = password {
            let _ = write!(encoder, "{password}");
        }
    }

    String::from_utf8(buf).expect("Failed to encode basic auth")
}

/// remove the username and password from the URL and return a new URL
/// with the username and password removed.
/// e.g: http://username:password@localhost:8080 -> http://localhost:8080
/// if the URL does not contain a username and password, the URL is returned
/// if authority is None, return the URL
pub(crate) fn remove_credentials(url: &Uri) -> Uri {
    let mut parts = url.clone().into_parts();

    if let Some(authority) = &parts.authority {
        if authority.as_str().contains('@') {
            let new_authority = authority
                .as_str()
                .split('@')
                .last()
                .expect("Failed to split authority");
            parts.authority = Some(new_authority.parse().unwrap());
        }
    }

    Uri::from_parts(parts).expect("Failed to build URI without credentials")
}

/// Check the request URL for a "username:password" type authority, and if
/// found, remove it from the URL and return it.
pub(crate) fn extract_authority(url: &Uri) -> Option<(String, Option<String>)> {
    let authority = url.authority();

    if let Some(_) = authority {
        let (username, password) = uri_credentials(&url);
        if let Some(username) = username {
            return Some((username, password));
        }
    }

    None
}

/// extrat decoded username and password from uri
/// e.g: username:password@example.com:123
/// return usernamem, password
fn uri_credentials(url: &Uri) -> (Option<String>, Option<String>) {
    use percent_encoding::percent_decode;

    let authority = url.authority().expect("authority is None");
    if !authority.as_str().contains('@') {
        return (None, None);
    }

    let mut credentials = authority
        .as_str()
        .split('@')
        .next()
        .unwrap_or("")
        .split(':');

    let username = credentials.next().and_then(|username| {
        if username.is_empty() {
            None
        } else {
            Some(
                percent_decode(username.as_bytes())
                    .decode_utf8()
                    .ok()?
                    .into(),
            )
        }
    });

    let password: Option<String> = credentials.next().and_then(|password| {
        if password.is_empty() {
            None
        } else {
            Some(
                percent_decode(password.as_bytes())
                    .decode_utf8()
                    .ok()?
                    .into(),
            )
        }
    });

    (username, password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_auth() {
        assert_eq!(basic_auth("user", None), "Basic dXNlcjo=");
        assert_eq!(
            basic_auth("user name", Some("password")),
            "Basic dXNlciBuYW1lOnBhc3N3b3Jk"
        );
    }

    #[test]
    fn test_extract_credentials() {
        let url = Uri::from_static("http://username:kevin%20meng@localhost:8080");
        let (username, password) = uri_credentials(&url);
        assert_eq!(username, Some("username".to_string()));
        assert_eq!(password, Some("kevin meng".to_string()));

        let url = Uri::from_static("http://username@localhost:8080");
        let (username, password) = uri_credentials(&url);
        assert_eq!(username, Some("username".to_string()));
        assert_eq!(password, None);

        let url = Uri::from_static("http://localhost:8080");
        let (username, password) = uri_credentials(&url);
        assert_eq!(username, None);
        assert_eq!(password, None);

        let url = Uri::from_static("http://@localhost:8080");
        let (username, password) = uri_credentials(&url);
        assert_eq!(username, None);
        assert_eq!(password, None);

        let url = Uri::from_static("http://%20:@localhost:8080");
        let (username, password) = uri_credentials(&url);
        assert_eq!(username, Some(" ".to_string()));
        assert_eq!(password, None);

        let url = Uri::from_static("http://super:@localhost:8080");
        let (username, password) = uri_credentials(&url);
        assert_eq!(username, Some("super".to_string()));
        assert_eq!(password, None);
    }

    #[test]
    fn test_extract_authority() {
        let mut url = Uri::from_static("http://username:kevin%20meng@localhost:8080");
        let (username, password) = extract_authority(&mut url).unwrap();
        assert_eq!(username, "username");
        assert_eq!(password, Some("kevin meng".to_string()));

        let mut url = Uri::from_static("http://username@localhost:8080");
        let (username, password) = extract_authority(&mut url).unwrap();
        assert_eq!(username, "username");
        assert_eq!(password, None);

        let mut url = Uri::from_static("http://localhost:8080");
        let credentials = extract_authority(&mut url);
        assert_eq!(credentials, None);

        let mut url = Uri::from_static("http://@localhost:8080");
        let credentials = extract_authority(&mut url);
        assert_eq!(credentials, None);

        let mut url = Uri::from_static("http://%20:@localhost:8080");
        let (username, password) = extract_authority(&mut url).unwrap();
        assert_eq!(username, " ".to_string());
        assert_eq!(password, None);

        let mut url = Uri::from_static("http://super:@localhost:8080");
        let (username, password) = extract_authority(&mut url).unwrap();
        assert_eq!(username, "super".to_string());
        assert_eq!(password, None);
    }

    #[test]
    fn test_remove_credentials() {
        let url = Uri::from_static("http://username:kevin%20meng@localhost:8080");
        let new_url = remove_credentials(&url);
        assert_eq!(new_url, Uri::from_static("http://localhost:8080"));

        let url = Uri::from_static("http://username@localhost:8080");
        let new_url = remove_credentials(&url);
        assert_eq!(new_url, Uri::from_static("http://localhost:8080"));

        let url = Uri::from_static("http://localhost:8080");
        let new_url = remove_credentials(&url);
        assert_eq!(new_url, Uri::from_static("http://localhost:8080"));

        let url = Uri::from_static("http://@localhost:8080");
        let new_url = remove_credentials(&url);
        assert_eq!(new_url, Uri::from_static("http://localhost:8080"));

        let url = Uri::from_static("http://%20:@localhost:8080");
        let new_url = remove_credentials(&url);
        assert_eq!(new_url, Uri::from_static("http://localhost:8080"));

        let url = Uri::from_static("http://super:@localhost:8080");
        let new_url = remove_credentials(&url);
        assert_eq!(new_url, Uri::from_static("http://localhost:8080"));
    }
}
