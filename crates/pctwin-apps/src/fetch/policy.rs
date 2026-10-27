//! Where a download may come from, checked before every connection (the 2.1 plan, section 9;
//! OWASP's SSRF guidance).
//!
//! - **Addresses.** Only `https`, a named host (never a bare IP address, which a web page could
//!   point at an internal machine), no user or password in the address, the usual port, no
//!   fragment, at most [`MAX_URL`] bytes. `file:`, `ftp:`, custom schemes and network shares are
//!   never fetched.
//! - **Where the name leads.** Every address a name resolves to must be on the public internet:
//!   not this laptop, the home or office network, the cloud's metadata service, link-local,
//!   carrier-grade NAT, multicast, documentation or reserved ranges, in IPv4, IPv6, or IPv4
//!   inside IPv6 (mapped, NAT64, 6to4, Teredo). One disallowed address refuses the name, and the
//!   connection uses only the addresses checked here (no second lookup that could be pointed
//!   elsewhere: T21-011).
//! - **Redirects.** At most [`MAX_REDIRECTS`]; each target is checked like the first address and
//!   must stay on the hosts the route allows (the same host, or the hosts a route names, such as
//!   GitHub's own download hosts). A redirect anywhere else stops the download (T21-008, T21-009).
//! - **No proxy, no fallback.** The system's proxy is not used (the address checks would not
//!   reach the real destination); a network that needs one ends with a plain reason. A TLS
//!   failure is a failure, never a retry without checking (T21-012).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

/// The longest address fetched.
pub const MAX_URL: usize = 2048;
/// Most redirects followed.
pub const MAX_REDIRECTS: usize = 5;

/// Why an address is not fetched.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refused {
    #[error("the address cannot be read")]
    Unreadable,
    #[error("only secure web addresses (https) are used")]
    NotHttps,
    #[error("the address names a computer by number, not by name")]
    BareAddress,
    #[error("the address carries a user name or password")]
    Credentials,
    #[error("the address uses an unusual port")]
    Port,
    #[error("the address is too long")]
    TooLong,
    #[error("the address leads to this laptop or a private network, not the internet")]
    Private,
    #[error("the download moved to a site the route does not allow")]
    OtherSite,
    #[error("the download moved too many times")]
    TooManyRedirects,
}

/// Checks one address (the first, or a redirect's target).
pub fn check_url(s: &str) -> Result<Url, Refused> {
    if s.len() > MAX_URL {
        return Err(Refused::TooLong);
    }
    let u = Url::parse(s).map_err(|_| Refused::Unreadable)?;
    if u.scheme() != "https" {
        return Err(Refused::NotHttps);
    }
    if !u.username().is_empty() || u.password().is_some() {
        return Err(Refused::Credentials);
    }
    match u.host() {
        Some(Host::Domain(d)) => {
            // A name made only of digits and dots is an address in disguise ("2130706433",
            // "0x7f.1"); the parser reads most as numbers already, this catches the rest.
            if d.split('.').all(|l| {
                l.chars()
                    .all(|c| c.is_ascii_hexdigit() || c == 'x' || c == 'X')
                    && l.chars().any(|c| c.is_ascii_digit())
            }) || d.ends_with(".localhost")
                || d == "localhost"
                || !d.contains('.')
            {
                return Err(Refused::BareAddress);
            }
        }
        Some(Host::Ipv4(_) | Host::Ipv6(_)) | None => return Err(Refused::BareAddress),
    }
    if u.port().is_some_and(|p| p != 443) {
        return Err(Refused::Port);
    }
    Ok(u)
}

fn v4_public(a: Ipv4Addr) -> bool {
    let o = a.octets();
    !(a.is_unspecified()
        || a.is_loopback()
        || a.is_private()
        || a.is_link_local()
        || a.is_broadcast()
        || a.is_multicast()
        || a.is_documentation()
        // 0.0.0.0/8, carrier-grade NAT 100.64/10, IETF 192.0.0/24, benchmarking 198.18/15,
        // reserved 240/4.
        || o[0] == 0
        || (o[0] == 100 && (o[1] & 0xc0) == 64)
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || (o[0] == 198 && (o[1] & 0xfe) == 18)
        || o[0] >= 240)
}

fn v6_public(a: Ipv6Addr) -> bool {
    if let Some(v4) = a.to_ipv4_mapped() {
        return v4_public(v4);
    }
    let s = a.segments();
    // NAT64 (64:ff9b::/96 and 64:ff9b:1::/48) carries an IPv4 address in its last 32 bits.
    if s[0] == 0x64 && s[1] == 0xff9b {
        let v4 = Ipv4Addr::new((s[6] >> 8) as u8, s[6] as u8, (s[7] >> 8) as u8, s[7] as u8);
        return v4_public(v4);
    }
    // 6to4 (2002::/16) carries one in bits 16 to 47.
    if s[0] == 0x2002 {
        let v4 = Ipv4Addr::new((s[1] >> 8) as u8, s[1] as u8, (s[2] >> 8) as u8, s[2] as u8);
        return v4_public(v4);
    }
    !(a.is_unspecified()
        || a.is_loopback()
        || a.is_multicast()
        // IPv4-compatible ::a.b.c.d (deprecated).
        || (s[0..6] == [0; 6])
        // Unique local fc00::/7, link-local fe80::/10, site-local fec0::/10.
        || (s[0] & 0xfe00) == 0xfc00
        || (s[0] & 0xffc0) == 0xfe80
        || (s[0] & 0xffc0) == 0xfec0
        // Teredo 2001::/32 (carries an address PCTwin does not unwrap), documentation
        // 2001:db8::/32 and 3fff::/20, discard 100::/64, ORCHID 2001:10::/28 and 2001:20::/28.
        || (s[0] == 0x2001 && s[1] == 0)
        || (s[0] == 0x2001 && s[1] == 0xdb8)
        || (s[0] & 0xfff0) == 0x3ff0
        || (s[0] == 0x100 && s[1..4] == [0; 3])
        || (s[0] == 0x2001 && (s[1] & 0xffe0) == 0x0020)
        || (s[0] == 0x2001 && (s[1] & 0xfff0) == 0x0010))
}

/// Whether an address is on the public internet.
pub fn address_ok(a: IpAddr) -> bool {
    match a {
        IpAddr::V4(v4) => v4_public(v4),
        IpAddr::V6(v6) => v6_public(v6),
    }
}

/// Every address a name resolved to must be public; one that is not refuses the name.
pub fn addresses_ok(found: &[IpAddr]) -> Result<(), Refused> {
    if found.is_empty() || !found.iter().all(|a| address_ok(*a)) {
        Err(Refused::Private)
    } else {
        Ok(())
    }
}

/// Which hosts a download may move to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hosts {
    /// Only the first address's own host.
    SameHost,
    /// The first address's host, or exactly one of these.
    These(Vec<String>),
}

impl Hosts {
    /// The hosts GitHub serves release files from.
    pub fn github() -> Hosts {
        Hosts::These(vec![
            "github.com".into(),
            "objects.githubusercontent.com".into(),
            "release-assets.githubusercontent.com".into(),
        ])
    }
}

/// The redirect rule for one download.
#[derive(Debug, Clone)]
pub struct Redirects {
    first_host: String,
    hosts: Hosts,
    followed: usize,
}

impl Redirects {
    pub fn new(first: &Url, hosts: Hosts) -> Self {
        Self {
            first_host: first.host_str().unwrap_or_default().to_ascii_lowercase(),
            hosts,
            followed: 0,
        }
    }

    /// Checks the next redirect's target.
    pub fn follow(&mut self, to: &str) -> Result<Url, Refused> {
        self.follow_with(to, check_url)
    }

    /// [`Redirects::follow`] with the address check `check` (tests use one that allows a local
    /// test server; the real one is always [`check_url`]).
    pub fn follow_with(
        &mut self,
        to: &str,
        check: fn(&str) -> Result<Url, Refused>,
    ) -> Result<Url, Refused> {
        self.followed += 1;
        if self.followed > MAX_REDIRECTS {
            return Err(Refused::TooManyRedirects);
        }
        let u = check(to)?;
        let host = u.host_str().unwrap_or_default().to_ascii_lowercase();
        let allowed = host == self.first_host
            || matches!(&self.hosts, Hosts::These(list) if list.iter().any(|h| h.eq_ignore_ascii_case(&host)));
        if allowed {
            Ok(u)
        } else {
            Err(Refused::OtherSite)
        }
    }
}

/// The address with its query and fragment removed: what is kept in records and reports (a
/// download link can carry a private token in its query: T21-051).
pub fn redacted(u: &Url) -> String {
    let mut c = u.clone();
    c.set_query(None);
    c.set_fragment(None);
    c.as_str().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_ordinary_secure_addresses_are_used() {
        assert!(check_url("https://download.example.com/a.msi").is_ok());
        assert!(check_url("https://download.example.com:443/a.msi").is_ok());
        assert_eq!(
            check_url("http://example.com/a.msi"),
            Err(Refused::NotHttps)
        );
        assert_eq!(
            check_url("file:///C:/Windows/notepad.exe"),
            Err(Refused::NotHttps)
        );
        assert_eq!(check_url("ftp://example.com/a"), Err(Refused::NotHttps));
        assert_eq!(check_url(r"\\server\share\a.msi"), Err(Refused::Unreadable));
        assert_eq!(
            check_url("https://user:pw@example.com/a"),
            Err(Refused::Credentials)
        );
        assert_eq!(check_url("https://example.com:8443/a"), Err(Refused::Port));
        assert_eq!(
            check_url(&format!("https://example.com/{}", "a".repeat(MAX_URL))),
            Err(Refused::TooLong)
        );
        assert_eq!(check_url("not an address"), Err(Refused::Unreadable));
    }

    #[test]
    fn addresses_in_disguise_are_refused() {
        for bare in [
            "https://127.0.0.1/",
            "https://169.254.169.254/latest/meta-data/",
            "https://[::1]/",
            "https://[::ffff:127.0.0.1]/",
            "https://2130706433/",
            "https://0x7f.0.0.1/",
            "https://0177.0.0.1/",
            "https://127.1/",
            "https://localhost/",
            "https://intranet/",
            "https://printer.localhost/",
        ] {
            assert!(check_url(bare).is_err(), "{bare}");
        }
    }

    #[test]
    fn only_public_addresses_are_connected_to() {
        let ok = |s: &str| address_ok(s.parse().unwrap());
        assert!(ok("93.184.216.34"));
        assert!(ok("2606:2800:220:1:248:1893:25c8:1946"));
        for private in [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "0.1.2.3",
            "255.255.255.255",
            "224.0.0.1",
            "192.0.0.170",
            "198.18.0.1",
            "192.0.2.1",
            "240.0.0.1",
            "::1",
            "::",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "fec0::1",
            "ff02::1",
            "::ffff:10.0.0.1",
            "::ffff:169.254.169.254",
            "64:ff9b::a9fe:a9fe",
            "2002:7f00:1::1",
            "2001:0:4136:e378::1",
            "2001:db8::1",
            "::7f00:1",
            "100::1",
        ] {
            assert!(!ok(private), "{private}");
        }
        // NAT64 and 6to4 of a public address are public.
        assert!(ok("64:ff9b::5db8:d822"));
        assert!(ok("2002:5db8:d822::1"));
        assert_eq!(addresses_ok(&[]), Err(Refused::Private));
        // One private answer among public ones refuses the name (a rebinding answer).
        assert_eq!(
            addresses_ok(&[
                "93.184.216.34".parse().unwrap(),
                "10.0.0.1".parse().unwrap()
            ]),
            Err(Refused::Private)
        );
    }

    #[test]
    fn redirects_stay_on_the_allowed_hosts_and_are_bounded() {
        let first = check_url("https://github.com/maker/tool/releases/download/v1/t.msi").unwrap();
        let mut r = Redirects::new(&first, Hosts::github());
        assert!(
            r.follow("https://release-assets.githubusercontent.com/x?token=1")
                .is_ok()
        );
        assert_eq!(
            r.follow("https://evil.example.net/t.msi"),
            Err(Refused::OtherSite)
        );
        assert_eq!(r.follow("http://github.com/x"), Err(Refused::NotHttps));
        assert_eq!(
            r.follow("https://169.254.169.254/"),
            Err(Refused::BareAddress)
        );
        // A multi-tenant host is the same host only when it is exactly the same (T21-008).
        let first = check_url("https://download.example.com/a").unwrap();
        let mut same = Redirects::new(&first, Hosts::SameHost);
        assert!(same.follow("https://download.example.com/b").is_ok());
        assert_eq!(
            same.follow("https://other.example.com/b"),
            Err(Refused::OtherSite)
        );
        assert_eq!(
            same.follow("https://download.example.com.evil.net/b"),
            Err(Refused::OtherSite)
        );
        let mut many = Redirects::new(&first, Hosts::SameHost);
        for _ in 0..MAX_REDIRECTS {
            many.follow("https://download.example.com/b").unwrap();
        }
        assert_eq!(
            many.follow("https://download.example.com/b"),
            Err(Refused::TooManyRedirects)
        );
    }

    #[test]
    fn private_parts_of_an_address_never_reach_a_record() {
        let u = check_url("https://dl.example.com/f/setup.exe?token=SECRET&user=ada#x").unwrap();
        assert_eq!(redacted(&u), "https://dl.example.com/f/setup.exe");
    }
}
