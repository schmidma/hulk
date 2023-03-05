use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CyclerConfiguration {
    pub name: String,
    pub kind: CyclerKind,
    pub instances: Option<Vec<String>>,
    pub module: String,
    pub nodes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CyclerKind {
    Perception,
    RealTime,
}
