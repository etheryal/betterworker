mod properties;

use betterworker_sys::IncomingRequestCfProperties;
pub use properties::{CfProperties, MinifyConfig, PolishConfig};
use send_wrapper::SendWrapper;

/// In addition to the methods on the `Request` struct, the `Cf` struct on an
/// inbound Request contains information about the request provided by
/// Cloudflare’s edge.
///
/// [Details](https://developers.cloudflare.com/workers/runtime-apis/request#incomingrequestcfproperties)
#[derive(Debug)]
pub struct Cf(SendWrapper<IncomingRequestCfProperties>);

impl Cf {
    pub fn new(inner: IncomingRequestCfProperties) -> Self {
        Self(SendWrapper::new(inner))
    }

    /// The three-letter airport code (e.g. `ATX`, `LUX`) representing
    /// the colocation which processed the request
    pub fn colo(&self) -> String {
        self.0.colo()
    }

    /// The Autonomous System Number (ASN) of the request, e.g. `395747`
    pub fn asn(&self) -> u32 {
        self.0.asn()
    }

    /// The two-letter country code of origin for the request.
    /// This is the same value as that provided in the CF-IPCountry header, e.g.
    /// `"US"`
    pub fn country(&self) -> Option<String> {
        self.0.country()
    }

    /// The HTTP Protocol (e.g. "HTTP/2") used by the request
    pub fn http_protocol(&self) -> String {
        self.0.http_protocol()
    }

    /// The browser-requested prioritization information in the request object,
    ///
    /// See [this blog post](https://blog.cloudflare.com/better-http-2-prioritization-for-a-faster-web/#customizingprioritizationwithworkers) for details.
    pub fn request_priority(&self) -> Option<RequestPriority> {
        if let Some(priority) = self.0.request_priority() {
            let mut weight = 1;
            let mut exclusive = false;
            let mut group = 0;
            let mut group_weight = 0;

            priority
                .as_str()
                .split(';')
                .map(|key_value_pair| {
                    let mut iter = key_value_pair.split('=');

                    // this pair is guaranteed to have 2 elements
                    let key = iter.next().unwrap(); // first element
                    let value = iter.next().unwrap(); // second element

                    (key, value)
                })
                .for_each(|(key, value)| match key {
                    "weight" => weight = value.parse().unwrap(),
                    "exclusive" => exclusive = value == "1",
                    "group" => group = value.parse().unwrap(),
                    "group-weight" => group_weight = value.parse().unwrap(),
                    _ => return,
                });

            Some(RequestPriority {
                weight,
                exclusive,
                group,
                group_weight,
            })
        } else {
            None
        }
    }

    /// The cipher for the connection to Cloudflare, e.g.
    /// "AEAD-AES128-GCM-SHA256".
    pub fn tls_cipher(&self) -> String {
        self.0.tls_cipher()
    }

    /// Information about the client's authorization.
    /// Only set when using Cloudflare Access or API Shield.
    pub fn tls_client_auth(&self) -> Option<TlsClientAuth> {
        self.0
            .tls_client_auth()
            .map(|inner| TlsClientAuth(SendWrapper::new(inner)))
    }

    /// The TLS version of the connection to Cloudflare, e.g. TLSv1.3.
    pub fn tls_version(&self) -> String {
        // TODO: should this be strongly typed? with ordering, etc.?
        self.0.tls_version()
    }

    /// City of the incoming request, e.g. "Austin".
    pub fn city(&self) -> Option<String> {
        self.0.city()
    }

    /// Continent of the incoming request, e.g. "NA"
    pub fn continent(&self) -> Option<String> {
        self.0.continent()
    }

    /// Latitude and longitude of the incoming request, e.g. (30.27130,
    /// -97.74260)
    pub fn coordinates(&self) -> Option<(f32, f32)> {
        let lat_opt = self.0.latitude();
        let lon_opt = self.0.longitude();
        match (lat_opt, lon_opt) {
            (Some(lat_str), Some(lon_str)) => {
                // SAFETY: i think this is fine..?
                let lat = lat_str.parse().unwrap();
                let lon = lon_str.parse().unwrap();
                Some((lat, lon))
            },
            _ => None,
        }
    }

    /// Postal code of the incoming request, e.g. "78701"
    pub fn postal_code(&self) -> Option<String> {
        self.0.postal_code()
    }

    /// Metro code (DMA) of the incoming request, e.g. "635"
    pub fn metro_code(&self) -> Option<String> {
        self.0.metro_code()
    }

    /// If known, the [ISO 3166-2](https://en.wikipedia.org/wiki/ISO_3166-2) name for the first level region associated with the IP address of the incoming request, e.g. "Texas".
    pub fn region(&self) -> Option<String> {
        self.0.region()
    }

    /// If known, the [ISO 3166-2](https://en.wikipedia.org/wiki/ISO_3166-2) code for the first level region associated with the IP address of the incoming request, e.g. "TX".
    pub fn region_code(&self) -> Option<String> {
        self.0.region_code()
    }

    /// Timezone of the incoming request
    pub fn timezone(&self) -> impl chrono::TimeZone {
        let tz = self.0.timezone();
        tz.parse::<chrono_tz::Tz>().unwrap()
    }

    /// Timezone name of the incoming request
    pub fn timezone_name(&self) -> String {
        self.0.timezone()
    }

    /// Whether the country of the incoming request is in the EU
    pub fn is_eu_country(&self) -> bool {
        self.0.is_eu_country() == Some("1".to_string())
    }
}

/// Browser-requested prioritization information.
#[derive(Debug, Clone, Copy)]
pub struct RequestPriority {
    /// The browser-requested weight for the HTTP/2 prioritization
    pub weight: usize,

    /// The browser-requested HTTP/2 exclusive flag (true for Chromium-based
    /// browsers, false for others).
    pub exclusive: bool,

    /// HTTP/2 stream ID for the request group (only non-zero for Firefox)
    pub group: usize,

    /// HTTP/2 weight for the request group (only non-zero for Firefox)
    pub group_weight: usize,
}

/// Only set when using Cloudflare Access or API Shield
#[derive(Debug)]
pub struct TlsClientAuth(SendWrapper<betterworker_sys::TlsClientAuth>);

impl TlsClientAuth {
    pub fn cert_issuer_dn_legacy(&self) -> String {
        self.0.cert_issuer_dn_legacy()
    }

    pub fn cert_issuer_dn(&self) -> String {
        self.0.cert_issuer_dn()
    }

    pub fn cert_issuer_dn_rfc2253(&self) -> String {
        self.0.cert_issuer_dn_rfc2253()
    }

    pub fn cert_subject_dn_legacy(&self) -> String {
        self.0.cert_subject_dn_legacy()
    }

    pub fn cert_verified(&self) -> String {
        self.0.cert_verified()
    }

    pub fn cert_not_after(&self) -> String {
        self.0.cert_not_after()
    }

    pub fn cert_subject_dn(&self) -> String {
        self.0.cert_subject_dn()
    }

    pub fn cert_fingerprint_sha1(&self) -> String {
        self.0.cert_fingerprint_sha1()
    }

    pub fn cert_not_before(&self) -> String {
        self.0.cert_not_before()
    }

    pub fn cert_serial(&self) -> String {
        self.0.cert_serial()
    }

    pub fn cert_presented(&self) -> String {
        self.0.cert_presented()
    }

    pub fn cert_subject_dn_rfc2253(&self) -> String {
        self.0.cert_subject_dn_rfc2253()
    }
}
