use std::collections::HashMap;

pub const AMBIENT_DESIGN_TOKENS_CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/ambient.css"));

include!(concat!(env!("OUT_DIR"), "/ambient.rs"));
