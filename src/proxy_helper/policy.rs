use hudsucker::hyper::Uri;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CanonicalDestination {
    pub(crate) scheme: String,
    pub(crate) host: String,
    pub(crate) port: u16,
}

impl CanonicalDestination {
    pub(crate) fn from_connect_uri(uri: &Uri) -> Result<Self, String> {
        let authority = uri
            .authority()
            .ok_or_else(|| "CONNECT request has no authority".to_string())?;
        let absolute = format!("https://{authority}")
            .parse::<Uri>()
            .map_err(|_| "CONNECT request has an invalid authority".to_string())?;
        Self::from_uri(&absolute)
    }

    pub(crate) fn from_uri(uri: &Uri) -> Result<Self, String> {
        let scheme = uri
            .scheme_str()
            .ok_or_else(|| "request has no scheme".to_string())?
            .to_ascii_lowercase();
        let default_port = match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => return Err("only HTTP and HTTPS destinations are supported".into()),
        };
        let authority = uri
            .authority()
            .ok_or_else(|| "request has no authority".to_string())?;
        if authority.as_str().contains('@') {
            return Err("destination userinfo is not supported".into());
        }
        let authority_host = authority.host().trim_end_matches('.');
        let host = authority_host
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .unwrap_or(authority_host)
            .to_ascii_lowercase();
        if host.is_empty() {
            return Err("request has an empty destination host".into());
        }
        if is_reserved_hostname(&host) {
            return Err("private or reserved destination is not allowed".into());
        }
        if let Ok(ip) = host.parse::<IpAddr>()
            && !is_public_ip(ip)
        {
            return Err("private or reserved destination is not allowed".into());
        }
        Ok(Self {
            scheme,
            host,
            port: authority.port_u16().unwrap_or(default_port),
        })
    }

    pub(crate) fn origin(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        let default_port = if self.scheme == "http" { 80 } else { 443 };
        if self.port == default_port {
            format!("{}://{host}", self.scheme)
        } else {
            format!("{}://{host}:{}", self.scheme, self.port)
        }
    }
}

pub(crate) fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let value = u32::from(ip);
    ![
        (0x0000_0000, 8),  // 0.0.0.0/8, current network
        (0x0a00_0000, 8),  // 10.0.0.0/8, private
        (0x6440_0000, 10), // 100.64.0.0/10, shared address space
        (0x7f00_0000, 8),  // 127.0.0.0/8, loopback
        (0xa9fe_0000, 16), // 169.254.0.0/16, link local
        (0xac10_0000, 12), // 172.16.0.0/12, private
        (0xc000_0000, 24), // 192.0.0.0/24, IETF protocol assignments
        (0xc000_0200, 24), // 192.0.2.0/24, documentation
        (0xc058_6300, 24), // 192.88.99.0/24, deprecated relay
        (0xc0a8_0000, 16), // 192.168.0.0/16, private
        (0xc612_0000, 15), // 198.18.0.0/15, benchmarking
        (0xc633_6400, 24), // 198.51.100.0/24, documentation
        (0xcb00_7100, 24), // 203.0.113.0/24, documentation
        (0xe000_0000, 4),  // 224.0.0.0/4, multicast
        (0xf000_0000, 4),  // 240.0.0.0/4, reserved and broadcast
    ]
    .into_iter()
    .any(|(network, bits)| value >> (32 - bits) == network >> (32 - bits))
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let value = u128::from(ip);
    if ip.is_unspecified() || ip.is_loopback() {
        return false;
    }
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    if value >> 32 == 0x0064_ff9b_0000_0000_0000_0000_0000_0000_u128 >> 32 {
        return is_public_ipv4(Ipv4Addr::from(value as u32));
    }
    if value >> 125 != 0b001 {
        return false;
    }
    ![
        (0x0064_ff9b_0001_0000_0000_0000_0000_0000_u128, 48), // local-use translation
        (0x0100_0000_0000_0000_0000_0000_0000_0000_u128, 64), // discard-only
        (0x2001_0000_0000_0000_0000_0000_0000_0000_u128, 23), // IETF protocol assignments
        (0x2001_0db8_0000_0000_0000_0000_0000_0000_u128, 32), // documentation
        (0x2002_0000_0000_0000_0000_0000_0000_0000_u128, 16), // 6to4
        (0x3fff_0000_0000_0000_0000_0000_0000_0000_u128, 20), // documentation
        (0xfc00_0000_0000_0000_0000_0000_0000_0000_u128, 7),  // unique local
        (0xfe80_0000_0000_0000_0000_0000_0000_0000_u128, 10), // link local
        (0xff00_0000_0000_0000_0000_0000_0000_0000_u128, 8),  // multicast
    ]
    .into_iter()
    .any(|(network, bits)| value >> (128 - bits) == network >> (128 - bits))
}

fn is_reserved_hostname(host: &str) -> bool {
    host == "localhost"
        || [
            ".localhost",
            ".local",
            ".internal",
            ".home.arpa",
            ".test",
            ".example",
            ".invalid",
            ".onion",
        ]
        .into_iter()
        .any(|suffix| host.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_exact_origins() {
        let destination =
            CanonicalDestination::from_uri(&"HTTPS://API.Example.COM.:443/v1?q=x".parse().unwrap())
                .unwrap();
        assert_eq!(destination.origin(), "https://api.example.com");
        let destination =
            CanonicalDestination::from_uri(&"https://api.example.com:8443/v1".parse().unwrap())
                .unwrap();
        assert_eq!(destination.origin(), "https://api.example.com:8443");
        let destination =
            CanonicalDestination::from_connect_uri(&"api.example.com:443".parse().unwrap())
                .unwrap();
        assert_eq!(destination.origin(), "https://api.example.com");
    }

    #[test]
    fn rejects_non_http_and_reserved_hosts() {
        for uri in [
            "file:///etc/passwd",
            "http://localhost/",
            "http://service.internal/",
            "http://127.0.0.1/",
            "http://169.254.169.254/latest/meta-data/",
            "http://10.0.0.1/",
            "http://[::1]/",
            "http://[fc00::1]/",
            "http://[::7f00:1]/",
            "http://[2001:db8::1]/",
            "http://[3fff::1]/",
            "http://[4000::1]/",
            "http://192.0.2.1/",
        ] {
            assert!(
                uri.parse::<Uri>()
                    .ok()
                    .and_then(|uri| CanonicalDestination::from_uri(&uri).ok())
                    .is_none(),
                "accepted {uri}"
            );
        }
    }

    #[test]
    fn accepts_public_addresses() {
        for address in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(is_public_ip(address.parse().unwrap()), "rejected {address}");
        }
    }
}
