//! Where the server listens: the override the record file names, else the
//! first address inside the mesh range, else nothing — which the composition
//! root shows as "not listening" rather than refusing to launch (`FR.5.1`).

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::StartError;

/// The port the phone server listens on when no override names another.
pub const DEFAULT_PHONE_PORT: u16 = 7466;

/// The network the mesh assigns its members from: `100.64.0.0/10`, the
/// carrier-grade NAT block no home router hands out, so an address in it can
/// only have come from the mesh.
const MESH_NETWORK: Ipv4Addr = Ipv4Addr::new(100, 64, 0, 0);

/// The prefix length of [`MESH_NETWORK`], in bits.
const MESH_PREFIX_BITS: u32 = 10;

/// The address to listen on: `override_` parsed when given (`<ip>:<port>`, or
/// a bare `<ip>` on [`DEFAULT_PHONE_PORT`]), else the first IPv4 address in
/// `100.64.0.0/10` among this machine's interfaces on [`DEFAULT_PHONE_PORT`].
///
/// # Errors
///
/// [`StartError::BadOverride`] when the override will not parse,
/// [`StartError::Interfaces`] when the interfaces cannot be listed, and
/// [`StartError::NoMeshAddress`] when none of them is inside the mesh range.
pub fn bind_address(override_: Option<&str>) -> Result<SocketAddr, StartError> {
    if let Some(text) = override_ {
        return parse_override(text);
    }
    let interfaces = if_addrs::get_if_addrs().map_err(|error| StartError::Interfaces {
        reason: error.to_string(),
    })?;
    let candidates = interfaces
        .into_iter()
        .filter_map(|interface| match interface.ip() {
            IpAddr::V4(address) => Some(address),
            IpAddr::V6(_) => None,
        });
    first_mesh_address(candidates)
}

/// The first of `candidates` inside the mesh range, on [`DEFAULT_PHONE_PORT`].
fn first_mesh_address(
    candidates: impl IntoIterator<Item = Ipv4Addr>,
) -> Result<SocketAddr, StartError> {
    candidates
        .into_iter()
        .find(|address| is_mesh_address(*address))
        .map(|address| SocketAddr::from((address, DEFAULT_PHONE_PORT)))
        .ok_or(StartError::NoMeshAddress)
}

fn parse_override(text: &str) -> Result<SocketAddr, StartError> {
    if let Ok(address) = text.parse::<SocketAddr>() {
        return Ok(address);
    }
    text.parse::<IpAddr>()
        .map(|ip| SocketAddr::from((ip, DEFAULT_PHONE_PORT)))
        .map_err(|_| StartError::BadOverride {
            text: text.to_owned(),
        })
}

fn is_mesh_address(address: Ipv4Addr) -> bool {
    let mask = u32::MAX << (32 - MESH_PREFIX_BITS);
    u32::from(address) & mask == u32::from(MESH_NETWORK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addresses(list: &[&str]) -> Vec<Ipv4Addr> {
        list.iter()
            .map(|text| text.parse().expect("a test address parses"))
            .collect()
    }

    #[test]
    fn the_override_wins_over_every_interface() {
        let chosen = bind_address(Some("192.168.1.20:9000")).expect("the override parses");

        assert_eq!(chosen, "192.168.1.20:9000".parse().expect("parses"));
    }

    #[test]
    fn an_override_without_a_port_listens_on_the_default_port() {
        let chosen = bind_address(Some("10.1.2.3")).expect("the override parses");

        assert_eq!(chosen, SocketAddr::from(([10, 1, 2, 3], 7466)));
    }

    #[test]
    fn an_override_that_is_not_an_address_is_refused_by_name() {
        let error = bind_address(Some("mesh")).expect_err("not an address");

        assert!(matches!(error, StartError::BadOverride { text } if text == "mesh"));
    }

    #[test]
    fn the_first_mesh_address_is_chosen_on_port_7466() {
        let candidates = addresses(&["127.0.0.1", "192.168.1.5", "100.101.7.9", "100.64.0.2"]);

        let chosen = first_mesh_address(candidates).expect("a mesh address is present");

        assert_eq!(chosen, SocketAddr::from(([100, 101, 7, 9], 7466)));
    }

    #[test]
    fn the_range_ends_at_100_127_255_255() {
        let candidates = addresses(&["100.63.255.255", "100.128.0.0", "100.127.255.255"]);

        let chosen = first_mesh_address(candidates).expect("the last one is inside");

        assert_eq!(chosen.ip(), IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255)));
    }

    #[test]
    fn no_mesh_address_among_the_interfaces_is_its_own_error() {
        let candidates = addresses(&["127.0.0.1", "192.168.1.5"]);

        let error = first_mesh_address(candidates).expect_err("nothing inside the range");

        assert!(matches!(error, StartError::NoMeshAddress));
    }
}
