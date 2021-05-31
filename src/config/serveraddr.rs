use super::Hostname;
use serde::{Deserialize, Serialize};
use std::{fmt, net::ToSocketAddrs, str::FromStr};

static DEFAULT_PORT_STR: &str = "25565";

#[derive(Debug, Clone)]
pub struct ServerAddr(Hostname, u16);

impl fmt::Display for ServerAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;
        write!(f, ":{}", self.1)?;

        Ok(())
    }
}

impl FromStr for ServerAddr {
    type Err = String;

    fn from_str(address: &str) -> Result<Self, Self::Err> {
        let mut parts = address.split(':');
        let hostname = parts.next().unwrap();
        let hostname = Hostname::from_str(&hostname)?;

        let port = parts
            .next()
            .unwrap_or(DEFAULT_PORT_STR)
            .parse::<u16>()
            .map_err(|err| {
                // ugly waiting for rust-lang#22639
                match err.to_string().as_str() {
                    // maybe just replace empty port with default?
                    // IntErrorKind::Empty
                    "cannot parse integer from empty string" => "port cannot be blank".to_owned(),

                    // IntErrorKind::InvalidDigit
                    "invalid digit found in string" => "port can only contain digits".to_owned(),

                    // IntErrorKind::Overflow
                    "number too large to fit in target type" => {
                        format!("port must be in range {}-{}", u16::MIN, u16::MAX)
                    }

                    // Underflow ('-' is invalid digit) and Zero don't apply to u16
                    _ => format!("error parsing port: {:?}", err),
                }
            })?;

        Ok(ServerAddr(hostname, port))
    }
}

impl ToSocketAddrs for ServerAddr {
    type Iter = std::vec::IntoIter<std::net::SocketAddr>;

    fn to_socket_addrs(&self) -> std::io::Result<Self::Iter> {
        (self.0.to_string().as_str(), self.1).to_socket_addrs()
    }
}

impl Serialize for ServerAddr {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ServerAddr {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let address = String::deserialize(deserializer)?;
        Self::from_str(&address).map_err(serde::de::Error::custom)
    }
}
