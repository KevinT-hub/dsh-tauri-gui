//! Fixed HTTPS geo endpoints. No tokens, no cookies, no user data — the
//! public IP of this machine is only used to learn the country code.

/// Public country-code endpoints that return the requester's ISO country
/// code. Each entry carries a stable name used in diagnostics.
pub const ENDPOINTS: &[(&str, &str)] = &[
    ("ipinfo", "https://ipinfo.io/json"),
    ("ipapi", "https://ipapi.co/json/"),
    ("country.is", "https://api.country.is/"),
];
