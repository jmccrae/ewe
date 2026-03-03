use serde::{Serialize,Deserialize};
#[cfg(feature="redb")]
use speedy::{Readable,Writable};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone,Eq,Hash,PartialOrd,Ord)]
#[cfg_attr(feature="redb", derive(Readable, Writable))]
pub struct DeprecationRecord(pub String,pub String,pub String,pub String,pub String);

