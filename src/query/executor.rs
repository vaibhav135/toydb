use std::error::Error;

use crate::query::parser::{ParsedQuery, QueryParser};

#[derive(Debug)]
pub struct QueryExecutor {
    query: String,
}

impl QueryExecutor {
    fn new(query: String) -> Self {
        QueryExecutor { query }
    }

    fn execute(&self) -> Result<(), Box<dyn Error>> {
        let qparser = ParsedQuery::default();
        let parsed_query = qparser.get_parsed_query(&self.query)?;

        Ok(())
    }
}
