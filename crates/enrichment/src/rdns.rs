//! Blocking reverse-DNS lookup helper.
//!
//! Only ever called from the enrichment worker thread, never from the
//! capture or UI hot paths.

use std::net::IpAddr;

/// Performs a blocking reverse-DNS lookup. Returns `None` if the lookup
/// fails or returns a name identical to the IP itself (some resolvers
/// return the input unchanged when they cannot resolve).
pub fn lookup(ip: IpAddr) -> Option<String> {
    let name = dns_lookup::lookup_addr(&ip).ok()?;
    if name.is_empty() || name == ip.to_string() {
        None
    } else {
        Some(name)
    }
}
