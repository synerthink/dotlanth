use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleAdditionContract {
    pub a: i32,
    pub b: i32,
}
