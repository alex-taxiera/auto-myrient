use once_cell::sync::Lazy;
use reqwest::header;
use std::collections::HashMap;

// Myrient HTTP-server addresses
pub static MYRIENT_HTTP_ADDR: &str = "https://myrient.erista.me/files/";

// Catalog URLs, to parse out the catalog in use from DAT
pub static CATALOG_URLS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    HashMap::from([
        ("https://www.no-intro.org", "No-Intro"),
        ("http://redump.org/", "Redump"),
    ])
});

// Postfixes in DATs to strip away
pub static DAT_POSTFIXES: Lazy<Vec<&str>> = Lazy::new(|| vec![" (Retool)"]);

// Headers to use in HTTP-requests
pub static REQ_HEADERS: Lazy<header::HeaderMap> = Lazy::new(|| {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"));
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"));

    headers
});
