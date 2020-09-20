#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleValues {
    keys: Vec<String>,
    values: Vec<String>,
}

impl ExampleValues {
    /// When no examples exist a vector with one empty ExampleValues struct is returned.
    pub fn from_examples(examples: &Option<gherkin::Examples>) -> Vec<ExampleValues> {
        match examples {
            Some(examples) => {
                let mut rows = Vec::with_capacity(examples.table.rows.len());
                for row_index in 1..examples.table.rows.len() {
                    rows.push(ExampleValues::new(
                        &examples.table.rows.first().unwrap().to_vec(),
                        &examples.table.rows.get(row_index).unwrap().to_vec(),
                    ))
                }
                rows
            }
            None => vec![ExampleValues::empty()],
        }
    }

    pub fn new(keys: &Vec<String>, values: &Vec<String>) -> ExampleValues {
        ExampleValues {
            keys: keys.into_iter().map(|val| format!("<{}>", val)).collect(),
            values: values.to_vec(),
        }
    }

    pub fn empty() -> ExampleValues {
        ExampleValues {
            keys: vec![],
            values: vec![],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn insert_values(&self, step: &String) -> String {
        let mut modified = step.to_owned();
        for index in 0..self.keys.len() {
            let search = self.keys.get(index).unwrap_or(&String::new()).to_owned();
            let replace_with = self.values.get(index).unwrap_or(&String::new()).to_owned();
            modified = modified.replace(&search, &replace_with);
        }
        modified
    }

    pub fn as_string(&self) -> String {
        let mut values = Vec::with_capacity(self.keys.len());
        for index in 0..self.keys.len() {
            values.push(format!(
                "{} = {}",
                self.keys.get(index).unwrap_or(&String::new()),
                self.values.get(index).unwrap_or(&String::new())
            ));
        }
        values.join(", ")
    }
}
