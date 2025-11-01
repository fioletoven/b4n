use std::fmt::Display;

/// Represents protocol type for the kubernetes resource's port definition.
#[derive(PartialEq)]
pub enum PortProtocol {
    TCP,
    UDP,
    SCTP,
}

impl Display for PortProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            PortProtocol::TCP => "TCP",
            PortProtocol::UDP => "UDP",
            PortProtocol::SCTP => "SCTP",
        };

        write!(f, "{value}")
    }
}

impl PortProtocol {
    /// Creates new [`PortProtocol`] instance.
    pub fn from(name: Option<&str>) -> Self {
        match name {
            Some(name) => {
                if name.eq_ignore_ascii_case("TCP") {
                    PortProtocol::TCP
                } else if name.eq_ignore_ascii_case("UDP") {
                    PortProtocol::UDP
                } else if name.eq_ignore_ascii_case("SCTP") {
                    PortProtocol::SCTP
                } else {
                    PortProtocol::TCP
                }
            },
            None => PortProtocol::TCP,
        }
    }
}

/// Represents kubernetes resource's port definition.
pub struct Port {
    pub port: u16,
    pub name: String,
    pub protocol: PortProtocol,
}
