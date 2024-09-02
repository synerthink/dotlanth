use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct SimpleAdditionContract {
    pub source_code: String,
}

impl SimpleAdditionContract {
    pub fn new() -> Self {
        let source_code = r#"
            let x = 10;
            let y = 5;
            let z = x + y;

        "#
        .to_string();

        SimpleAdditionContract { source_code }
    }
}
