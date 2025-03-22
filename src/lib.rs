use dashmap::{DashMap, DashSet};
use once_cell::sync::Lazy;

pub static CONNECTED_CLIENTS: Lazy<DashMap<String, flume::Sender<String>>> =
    Lazy::new(|| DashMap::new());
pub static USERNAME_UUID_MAP: Lazy<DashMap<String, DashSet<String>>> = Lazy::new(|| DashMap::new());

pub mod error;
