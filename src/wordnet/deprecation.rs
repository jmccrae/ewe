use serde::{Serialize,Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
pub struct DeprecationRecord(pub String,pub String,pub String,pub String,pub String);

