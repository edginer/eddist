use serde::{Deserialize, Serialize};

use super::repository::{CreatingRes, CreatingThread};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PubSubItem {
    CreatingRes(Box<CreatingRes>),
    CreatingThread(Box<CreatingThread>),
    CreatingResWhenFailed(Box<CreatingRes>),
    PersistenceShutdown,
}
