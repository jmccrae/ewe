use serde::{Serialize,Deserialize};
use std::io::Write;
use crate::wordnet::util::escape_yaml_string;

#[derive(Debug, PartialEq, Serialize, Deserialize,Clone)]
pub struct Pronunciation {
    value : String,
    variety : Option<String>
}

impl Pronunciation {
    pub(crate) fn save<W : Write>(&self, w : &mut W) -> std::io::Result<()> {
        write!(w, "\n    - value: {}", escape_yaml_string(&self.value, 6, 6))?;
        match &self.variety {
            Some(v) => {
                write!(w, "\n      variety: {}", v)?;
            }
            None => {}
        }
        Ok(())
    }
}


