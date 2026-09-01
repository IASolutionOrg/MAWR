use std::collections::BTreeSet;
use std::fmt;
use std::net::IpAddr;
use std::num::{NonZeroU16, NonZeroU64, NonZeroUsize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_CERTIFICATE_BYTES: usize = 64 * 1024;
const MAX_EXPLICIT_ADDRESSES: usize = 256;
const MAX_ALLOWED_PORTS: usize = 32;
const MAX_FILENAME_BYTES: usize = 240;
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
const MAX_HEADER_BYTES: u64 = 16 * 1024 * 1024;
const MAX_FORM_BYTES: u64 = 16 * 1024 * 1024;
const MAX_COOKIE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_HEADER_COUNT: usize = 4_096;
const MAX_REDIRECTS: usize = 100;
const MAX_DNS_ADDRESSES: usize = 256;
const MAX_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    Zero,
    OutOfRange,
    TooManyValues,
    InvalidCertificate,
    InvalidDownloadRoot,
    DownloadRootIsNotDirectory,
    UnsafeFilename,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Zero => "a configured numeric value must be non-zero",
            Self::OutOfRange => "a configured numeric value exceeds its supported range",
            Self::TooManyValues => "a configured collection exceeds its limit",
            Self::InvalidCertificate => "a configured TLS certificate is invalid",
            Self::InvalidDownloadRoot => "the download root cannot be resolved safely",
            Self::DownloadRootIsNotDirectory => "the download root is not a directory",
            Self::UnsafeFilename => "the download filename is unsafe",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportLimits {
    max_response_bytes: NonZeroU64,
    max_download_bytes: NonZeroU64,
    max_header_bytes: NonZeroU64,
    max_header_count: NonZeroUsize,
    max_form_bytes: NonZeroU64,
    max_cookie_bytes: NonZeroU64,
    max_redirects: NonZeroUsize,
    max_dns_addresses: NonZeroUsize,
    connect_timeout: Duration,
    read_timeout: Duration,
    total_timeout: Duration,
}

impl TransportLimits {
    pub fn with_max_response_bytes(mut self, value: u64) -> Result<Self, ConfigError> {
        self.max_response_bytes = bounded_u64(value, MAX_RESPONSE_BYTES)?;
        Ok(self)
    }

    pub fn with_max_download_bytes(mut self, value: u64) -> Result<Self, ConfigError> {
        self.max_download_bytes = bounded_u64(value, MAX_DOWNLOAD_BYTES)?;
        Ok(self)
    }

    pub fn with_max_header_bytes(mut self, value: u64) -> Result<Self, ConfigError> {
        self.max_header_bytes = bounded_u64(value, MAX_HEADER_BYTES)?;
        Ok(self)
    }

    pub fn with_max_header_count(mut self, value: usize) -> Result<Self, ConfigError> {
        if value > MAX_HEADER_COUNT {
            return Err(ConfigError::OutOfRange);
        }
        self.max_header_count = NonZeroUsize::new(value).ok_or(ConfigError::Zero)?;
        Ok(self)
    }

    pub fn with_max_form_bytes(mut self, value: u64) -> Result<Self, ConfigError> {
        self.max_form_bytes = bounded_u64(value, MAX_FORM_BYTES)?;
        Ok(self)
    }

    pub fn with_max_cookie_bytes(mut self, value: u64) -> Result<Self, ConfigError> {
        self.max_cookie_bytes = bounded_u64(value, MAX_COOKIE_BYTES)?;
        Ok(self)
    }

    pub fn with_max_redirects(mut self, value: usize) -> Result<Self, ConfigError> {
        if value > MAX_REDIRECTS {
            return Err(ConfigError::OutOfRange);
        }
        self.max_redirects = NonZeroUsize::new(value).ok_or(ConfigError::Zero)?;
        Ok(self)
    }

    pub fn with_max_dns_addresses(mut self, value: usize) -> Result<Self, ConfigError> {
        if value > MAX_DNS_ADDRESSES {
            return Err(ConfigError::OutOfRange);
        }
        self.max_dns_addresses = NonZeroUsize::new(value).ok_or(ConfigError::Zero)?;
        Ok(self)
    }

    pub fn with_connect_timeout(mut self, value: Duration) -> Result<Self, ConfigError> {
        self.connect_timeout = nonzero_duration(value)?;
        Ok(self)
    }

    pub fn with_read_timeout(mut self, value: Duration) -> Result<Self, ConfigError> {
        self.read_timeout = nonzero_duration(value)?;
        Ok(self)
    }

    pub fn with_total_timeout(mut self, value: Duration) -> Result<Self, ConfigError> {
        self.total_timeout = nonzero_duration(value)?;
        Ok(self)
    }

    pub(crate) const fn max_response_bytes(&self) -> u64 {
        self.max_response_bytes.get()
    }

    pub(crate) const fn max_download_bytes(&self) -> u64 {
        self.max_download_bytes.get()
    }

    pub(crate) const fn max_header_bytes(&self) -> u64 {
        self.max_header_bytes.get()
    }

    pub(crate) const fn max_header_count(&self) -> usize {
        self.max_header_count.get()
    }

    pub(crate) const fn max_form_bytes(&self) -> u64 {
        self.max_form_bytes.get()
    }

    pub(crate) const fn max_cookie_bytes(&self) -> u64 {
        self.max_cookie_bytes.get()
    }

    pub(crate) const fn max_redirects(&self) -> usize {
        self.max_redirects.get()
    }

    pub(crate) const fn max_dns_addresses(&self) -> usize {
        self.max_dns_addresses.get()
    }

    pub(crate) const fn connect_timeout(&self) -> Duration {
        self.connect_timeout
    }

    pub(crate) const fn read_timeout(&self) -> Duration {
        self.read_timeout
    }

    pub(crate) const fn total_timeout(&self) -> Duration {
        self.total_timeout
    }
}

impl Default for TransportLimits {
    fn default() -> Self {
        Self {
            max_response_bytes: NonZeroU64::new(8 * 1024 * 1024).expect("constant is non-zero"),
            max_download_bytes: NonZeroU64::new(64 * 1024 * 1024).expect("constant is non-zero"),
            max_header_bytes: NonZeroU64::new(64 * 1024).expect("constant is non-zero"),
            max_header_count: NonZeroUsize::new(256).expect("constant is non-zero"),
            max_form_bytes: NonZeroU64::new(256 * 1024).expect("constant is non-zero"),
            max_cookie_bytes: NonZeroU64::new(64 * 1024).expect("constant is non-zero"),
            max_redirects: NonZeroUsize::new(10).expect("constant is non-zero"),
            max_dns_addresses: NonZeroUsize::new(16).expect("constant is non-zero"),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(15),
            total_timeout: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AddressRule {
    PublicInternet,
    LoopbackOnly,
    Explicit(BTreeSet<IpAddr>),
}

/// Destination authorization applied after URL parsing and DNS resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationPolicy {
    address_rule: AddressRule,
    allowed_ports: BTreeSet<NonZeroU16>,
}

impl DestinationPolicy {
    #[must_use]
    pub fn public_web() -> Self {
        Self {
            address_rule: AddressRule::PublicInternet,
            allowed_ports: [80_u16, 443]
                .into_iter()
                .map(|port| NonZeroU16::new(port).expect("constant is non-zero"))
                .collect(),
        }
    }

    pub fn loopback(port: u16) -> Result<Self, ConfigError> {
        Ok(Self {
            address_rule: AddressRule::LoopbackOnly,
            allowed_ports: [NonZeroU16::new(port).ok_or(ConfigError::Zero)?]
                .into_iter()
                .collect(),
        })
    }

    pub fn explicit(
        addresses: impl IntoIterator<Item = IpAddr>,
        ports: impl IntoIterator<Item = u16>,
    ) -> Result<Self, ConfigError> {
        let addresses = addresses.into_iter().collect::<BTreeSet<_>>();
        let ports = ports
            .into_iter()
            .map(|port| NonZeroU16::new(port).ok_or(ConfigError::Zero))
            .collect::<Result<BTreeSet<_>, _>>()?;
        if addresses.is_empty() || ports.is_empty() {
            return Err(ConfigError::Zero);
        }
        if addresses.len() > MAX_EXPLICIT_ADDRESSES || ports.len() > MAX_ALLOWED_PORTS {
            return Err(ConfigError::TooManyValues);
        }
        Ok(Self {
            address_rule: AddressRule::Explicit(addresses),
            allowed_ports: ports,
        })
    }

    pub(crate) fn allows_port(&self, port: u16) -> bool {
        NonZeroU16::new(port).is_some_and(|port| self.allowed_ports.contains(&port))
    }

    pub(crate) fn allows_address(&self, address: IpAddr) -> bool {
        match &self.address_rule {
            AddressRule::PublicInternet => is_public_address(address),
            AddressRule::LoopbackOnly => address.is_loopback(),
            AddressRule::Explicit(addresses) => addresses.contains(&address),
        }
    }
}

impl Default for DestinationPolicy {
    fn default() -> Self {
        Self::public_web()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DerCertificate(Vec<u8>);

impl DerCertificate {
    pub fn new(bytes: Vec<u8>) -> Result<Self, ConfigError> {
        if bytes.is_empty() || bytes.len() > MAX_CERTIFICATE_BYTES {
            return Err(ConfigError::InvalidCertificate);
        }
        reqwest::tls::Certificate::from_der(&bytes).map_err(|_| ConfigError::InvalidCertificate)?;
        Ok(Self(bytes))
    }

    pub(crate) fn to_reqwest(&self) -> reqwest::tls::Certificate {
        reqwest::tls::Certificate::from_der(&self.0)
            .expect("DerCertificate validates its bytes at construction")
    }
}

impl fmt::Debug for DerCertificate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DerCertificate")
            .field("bytes", &self.0.len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TlsTrust {
    #[default]
    Platform,
    Only(Vec<DerCertificate>),
}

impl TlsTrust {
    pub fn only(certificates: Vec<DerCertificate>) -> Result<Self, ConfigError> {
        if certificates.is_empty() || certificates.len() > 32 {
            return Err(ConfigError::TooManyValues);
        }
        Ok(Self::Only(certificates))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NativeStaticConfig {
    limits: TransportLimits,
    destination_policy: DestinationPolicy,
    tls_trust: TlsTrust,
}

impl NativeStaticConfig {
    #[must_use]
    pub fn with_limits(mut self, limits: TransportLimits) -> Self {
        self.limits = limits;
        self
    }

    #[must_use]
    pub fn with_destination_policy(mut self, destination_policy: DestinationPolicy) -> Self {
        self.destination_policy = destination_policy;
        self
    }

    #[must_use]
    pub fn with_tls_trust(mut self, tls_trust: TlsTrust) -> Self {
        self.tls_trust = tls_trust;
        self
    }

    pub(crate) const fn limits(&self) -> &TransportLimits {
        &self.limits
    }

    pub(crate) const fn destination_policy(&self) -> &DestinationPolicy {
        &self.destination_policy
    }

    pub(crate) const fn tls_trust(&self) -> &TlsTrust {
        &self.tls_trust
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SafeFilename(String);

impl SafeFilename {
    pub fn new(value: impl Into<String>) -> Result<Self, ConfigError> {
        let value = value.into();
        let lower = value.to_ascii_lowercase();
        let stem = lower.split('.').next().unwrap_or("");
        let reserved = matches!(stem, "con" | "prn" | "aux" | "nul")
            || (stem.len() == 4
                && (stem.starts_with("com") || stem.starts_with("lpt"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'));
        if value.is_empty()
            || value.len() > MAX_FILENAME_BYTES
            || value == "."
            || value == ".."
            || value.ends_with(['.', ' '])
            || reserved
            || value.chars().any(|character| {
                character.is_control()
                    || matches!(
                        character,
                        '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                    )
            })
        {
            return Err(ConfigError::UnsafeFilename);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SafeFilename {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SafeFilename(<redacted>)")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DownloadPolicy {
    root: PathBuf,
    max_bytes: NonZeroU64,
}

impl DownloadPolicy {
    pub fn new(root: impl AsRef<Path>, max_bytes: u64) -> Result<Self, ConfigError> {
        let root = std::fs::canonicalize(root).map_err(|_| ConfigError::InvalidDownloadRoot)?;
        if !root.is_dir() {
            return Err(ConfigError::DownloadRootIsNotDirectory);
        }
        Ok(Self {
            root,
            max_bytes: nonzero_u64(max_bytes)?,
        })
    }

    pub(crate) const fn root(&self) -> &PathBuf {
        &self.root
    }

    pub(crate) const fn max_bytes(&self) -> u64 {
        self.max_bytes.get()
    }
}

impl fmt::Debug for DownloadPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DownloadPolicy")
            .field("root", &"<redacted>")
            .field("max_bytes", &self.max_bytes)
            .finish()
    }
}

fn nonzero_u64(value: u64) -> Result<NonZeroU64, ConfigError> {
    NonZeroU64::new(value).ok_or(ConfigError::Zero)
}

fn nonzero_duration(value: Duration) -> Result<Duration, ConfigError> {
    if value.is_zero() {
        Err(ConfigError::Zero)
    } else if value > MAX_TIMEOUT {
        Err(ConfigError::OutOfRange)
    } else {
        Ok(value)
    }
}

fn bounded_u64(value: u64, maximum: u64) -> Result<NonZeroU64, ConfigError> {
    if value > maximum {
        Err(ConfigError::OutOfRange)
    } else {
        nonzero_u64(value)
    }
}

fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            let [a, b, c, _] = address.octets();
            !(a == 0
                || a == 10
                || a == 127
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 0 && c == 2)
                || (a == 192 && b == 168)
                || (a == 192 && b == 88 && c == 99)
                || (a == 198 && (b == 18 || b == 19))
                || (a == 198 && b == 51 && c == 100)
                || (a == 203 && b == 0 && c == 113)
                || a >= 224)
        }
        IpAddr::V6(address) => {
            if let Some(mapped) = address.to_ipv4_mapped() {
                return is_public_address(IpAddr::V4(mapped));
            }
            let octets = address.octets();
            // Fail closed to the currently assignable global-unicast block, then
            // remove special-purpose ranges embedded within it. This also rejects
            // IPv4-compatible, translation, 6to4, local, multicast, and future
            // reserved forms without attempting to recover an embedded IPv4 value.
            (octets[0] & 0xe0) == 0x20
                && !(octets[0] == 0x20 && octets[1] == 0x01 && octets[2] < 0x02)
                && octets[..4] != [0x20, 0x01, 0x0d, 0xb8]
                && octets[..2] != [0x20, 0x02]
                && !(octets[0] == 0x3f && (octets[1] & 0xf0) == 0xf0)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::{DestinationPolicy, SafeFilename, TransportLimits};

    #[test]
    fn public_policy_rejects_special_address_ranges() {
        let policy = DestinationPolicy::public_web();
        for address in [
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 88, 99, 1)),
            IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1)),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
            IpAddr::V6("::c0a8:101".parse().unwrap()),
            IpAddr::V6("::ffff:c0a8:101".parse().unwrap()),
            IpAddr::V6("64:ff9b::c0a8:101".parse().unwrap()),
            IpAddr::V6("64:ff9b:1::c0a8:101".parse().unwrap()),
            IpAddr::V6("100::1".parse().unwrap()),
            IpAddr::V6("100:0:0:1::1".parse().unwrap()),
            IpAddr::V6("2001::1".parse().unwrap()),
            IpAddr::V6("fe80::1".parse().unwrap()),
            IpAddr::V6("fd00::1".parse().unwrap()),
            IpAddr::V6("2001:db8::1".parse().unwrap()),
            IpAddr::V6("2002:c0a8:101::".parse().unwrap()),
            IpAddr::V6("3fff::1".parse().unwrap()),
            IpAddr::V6("5f00::1".parse().unwrap()),
        ] {
            assert!(!policy.allows_address(address), "accepted {address}");
        }
        assert!(policy.allows_address(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
        assert!(policy.allows_address(IpAddr::V6("2606:2800:220:1::1".parse().unwrap())));
        let loopback = DestinationPolicy::loopback(8080).unwrap();
        assert!(loopback.allows_address(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    }

    #[test]
    fn safe_filename_rejects_traversal_and_platform_devices() {
        for value in [
            "",
            ".",
            "..",
            "../secret",
            "a/b",
            "a\\b",
            "a:b",
            "CON",
            "lpt1.txt",
        ] {
            assert!(SafeFilename::new(value).is_err(), "accepted {value:?}");
        }
        assert!(SafeFilename::new("report-2026.html").is_ok());
    }

    #[test]
    fn transport_limits_reject_zero_and_excessive_configuration() {
        assert!(TransportLimits::default().with_max_redirects(0).is_err());
        assert!(TransportLimits::default().with_max_redirects(101).is_err());
        assert!(
            TransportLimits::default()
                .with_max_response_bytes(1024 * 1024 * 1024 + 1)
                .is_err()
        );
        assert!(
            TransportLimits::default()
                .with_max_response_bytes(1)
                .is_ok()
        );
    }
}
