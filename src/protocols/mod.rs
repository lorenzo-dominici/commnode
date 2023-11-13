//! This module aggregates all the modules defining and managing their own protocols.

use std::fmt::Display;

use serde::{Serialize, Deserialize};

pub mod tcp;
pub mod udp;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Self::TCP => "TCP",
            Self::UDP => "UDP",
        };
        write!(f, "{}", string)
    }
}