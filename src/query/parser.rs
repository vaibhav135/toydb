use std::{error::Error, fmt::Debug, str::FromStr};

use crate::query::common::{QueryClause, QueryType};

#[derive(Debug, Default)]
pub struct ParsedQuery {
    pub tblname: String,
    pub output_fields: Vec<String>,
    pub query_type: QueryType,
}

pub trait QueryParser {
    fn get_parsed_query(&self, query: &str) -> Result<ParsedQuery, Box<dyn Error>>;
}

impl QueryParser for ParsedQuery {
    fn get_parsed_query(&self, query: &str) -> Result<ParsedQuery, Box<dyn Error>> {
        let query_tokens: Vec<String> = query
            .split(' ')
            .map(|s| s.to_lowercase().to_string())
            .collect();

        let query_type = QueryType::from_str(&query_tokens[0])?;
        let mut output_fields: Vec<String> = vec![];

        let from_clause: String = QueryClause::From.into();

        let from_pos = query_tokens.iter().position(|token| token == &from_clause);

        if from_pos.is_none() {
            return Err(format!(
                "FROM/from clause is missing. Please enter you query again with from clause"
            )
            .into());
        };

        output_fields = query_tokens[1..from_pos.unwrap()].to_vec();

        if query_tokens.get(from_pos.unwrap() + 1).is_none() {
            return Err(format!(
                "You forgot to add table name in your query, please provide table name"
            )
            .into());
        };

        let tblname = query_tokens[from_pos.unwrap() + 1].to_owned();

        return Ok(ParsedQuery {
            tblname,
            output_fields,
            query_type,
        });
    }
}
