use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub type Filter = fn(IpAddr) -> Option<&'static str>;

const NAT64_PREFIX: [u16; 6] = [0x64, 0xff9b, 0, 0, 0, 0];
const SIX_TO_FOUR: u16 = 0x2002;

fn v4_refusal(address: Ipv4Addr) -> Option<&'static str> {
    let [first, second, ..] = address.octets();
    if first == 0 {
        return Some("unspecified");
    }
    if address.is_loopback() {
        return Some("loopback");
    }
    if address.is_link_local() {
        return Some("link-local");
    }
    if address.is_private() {
        return Some("private");
    }
    if first == 100 && (64..128).contains(&second) {
        return Some("shared");
    }
    if address.is_multicast() {
        return Some("multicast");
    }
    if first >= 240 {
        return Some("reserved");
    }
    None
}

fn embedded_v4(address: Ipv6Addr) -> Option<Ipv4Addr> {
    let segments = address.segments();
    let from = |high: u16, low: u16| Ipv4Addr::from((u32::from(high) << 16) | u32::from(low));
    if let Some(mapped) = address.to_ipv4_mapped() {
        return Some(mapped);
    }
    if segments[..6] == NAT64_PREFIX {
        return Some(from(segments[6], segments[7]));
    }
    if segments[0] == SIX_TO_FOUR {
        return Some(from(segments[1], segments[2]));
    }
    if segments[..6] == [0; 6] {
        return Some(from(segments[6], segments[7]));
    }
    None
}

fn v6_refusal(address: Ipv6Addr) -> Option<&'static str> {
    if address.is_unspecified() {
        return Some("unspecified");
    }
    if address.is_loopback() {
        return Some("loopback");
    }
    if let Some(embedded) = embedded_v4(address) {
        return v4_refusal(embedded);
    }
    let first = address.segments()[0];
    if first & 0xfe00 == 0xfc00 {
        return Some("private");
    }
    if first & 0xffc0 == 0xfe80 {
        return Some("link-local");
    }
    if first & 0xff00 == 0xff00 {
        return Some("multicast");
    }
    None
}

pub fn refusal(address: IpAddr) -> Option<&'static str> {
    match address {
        IpAddr::V4(address) => v4_refusal(address),
        IpAddr::V6(address) => v6_refusal(address),
    }
}

pub fn unfiltered(_: IpAddr) -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(address: &str) -> Option<&'static str> {
        refusal(address.parse().unwrap())
    }

    #[test]
    fn public_addresses_are_let_through() {
        for address in [
            "1.1.1.1",
            "93.184.216.34",
            "172.32.0.1",
            "100.128.0.1",
            "2606:4700:4700::1111",
            "::ffff:8.8.8.8",
            "64:ff9b::808:808",
        ] {
            assert_eq!(class(address), None, "{address}");
        }
    }

    #[test]
    fn loopback_addresses_are_refused() {
        for address in ["127.0.0.1", "127.255.255.254", "::1"] {
            assert_eq!(class(address), Some("loopback"), "{address}");
        }
    }

    #[test]
    fn link_local_addresses_are_refused() {
        for address in ["169.254.169.254", "fe80::1", "febf::1"] {
            assert_eq!(class(address), Some("link-local"), "{address}");
        }
    }

    #[test]
    fn private_addresses_are_refused() {
        for address in [
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.1.1",
            "fc00::1",
            "fd12:3456::1",
        ] {
            assert_eq!(class(address), Some("private"), "{address}");
        }
    }

    #[test]
    fn unspecified_and_multicast_addresses_are_refused() {
        assert_eq!(class("0.0.0.0"), Some("unspecified"));
        assert_eq!(class("0.1.2.3"), Some("unspecified"));
        assert_eq!(class("::"), Some("unspecified"));
        assert_eq!(class("224.0.0.1"), Some("multicast"));
        assert_eq!(class("ff02::1"), Some("multicast"));
    }

    #[test]
    fn shared_and_reserved_ranges_are_refused() {
        assert_eq!(class("100.64.0.1"), Some("shared"));
        assert_eq!(class("100.127.255.255"), Some("shared"));
        assert_eq!(class("240.0.0.1"), Some("reserved"));
        assert_eq!(class("255.255.255.255"), Some("reserved"));
    }

    #[test]
    fn an_ipv4_address_inside_an_ipv6_one_is_judged_as_the_ipv4_address() {
        assert_eq!(class("::ffff:127.0.0.1"), Some("loopback"));
        assert_eq!(class("::ffff:10.0.0.1"), Some("private"));
        assert_eq!(class("::ffff:169.254.169.254"), Some("link-local"));
        assert_eq!(class("::ffff:0.0.0.0"), Some("unspecified"));
        assert_eq!(class("::ffff:224.0.0.1"), Some("multicast"));
        assert_eq!(class("64:ff9b::7f00:1"), Some("loopback"));
        assert_eq!(class("2002:c0a8:101::1"), Some("private"));
        assert_eq!(class("::10.0.0.1"), Some("private"));
    }

    #[test]
    fn the_lifted_filter_lets_every_address_through() {
        assert_eq!(unfiltered("127.0.0.1".parse().unwrap()), None);
    }
}
