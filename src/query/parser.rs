use std::{error::Error, str::FromStr};

use crate::{
    btree::SchemaType,
    query::common::{
        ColType, CreateParsedQuery, ParsedQueryResult, QueryClause, QueryType, SelectParsedQuery,
    },
    record_type::RecordDataType,
};

trait QueryParserInner {
    fn get_col_name(&self, query_tokens: Vec<String>) -> Result<Vec<String>, Box<dyn Error>> {
        let mut cols: Vec<String> = vec![];
        let col_start_idx = query_tokens.iter().position(|t| t.contains("(")).unwrap();
        let col_end_idx = query_tokens.iter().position(|t| t.contains(")")).unwrap();

        // NOTE:
        // Currently we don't support the select stmt (inside create) so I am going to ignore that.
        // I know  it's a bad move ("bad practise"). But it's a dumb parser so no need to worry.
        // TODO: Add support in the future maybe.
        let col_tokens = query_tokens[col_start_idx..=col_end_idx].to_vec();

        let mut field_name = String::from("");

        if col_start_idx == col_end_idx {
            let elem = self.filter_delim(cols[0].to_string());

            let mut colname;

            if elem.len() > 1 {
                colname = elem[elem.len() - 1].to_string();
            } else {
                colname = elem[0].to_string();
            }
            colname = colname.replace(")", "");

            cols.push(colname.to_string());
        } else {
            for (idx, t) in col_tokens.iter().enumerate() {
                if t.contains("(") || t.contains(")") || t.contains(",") {
                    let elem = self.filter_delim(t.to_string());

                    if elem.len() > 0 && field_name.is_empty() {
                        if t.contains("(") {
                            // This is for the case if someone write the query as
                            // tbl_name(col_name.
                            field_name = elem[elem.len() - 1].to_string();
                        } else {
                            field_name = elem[0].to_string();
                        }
                    }
                } else {
                    if field_name.is_empty() {
                        field_name = t.to_string();
                    }
                }

                if t.contains(",") || col_tokens.len() - 1 == idx {
                    if !field_name.is_empty() {
                        cols.push(field_name);
                        field_name = String::from("");
                    } else if field_name.is_empty() {
                        return Err(format!(
                            "Invalid query no col / field found for this create query"
                        )
                        .into());
                    } else {
                        cols.push(field_name);
                        field_name = String::from("");
                    }
                }
            }
        }

        Ok(cols)
    }

    fn filter_delim(&self, token: String) -> Vec<String> {
        if token.len() <= 1 {
            return vec![];
        };

        let filt = |token: &str, delim: &str| -> Vec<String> {
            token
                .to_string()
                .split(delim)
                .map(|s| s.to_string())
                .filter(|s| s != "")
                .collect::<Vec<String>>()
        };

        match token.trim() {
            token if token.contains("(") => filt(token, "("),
            token if token.contains(")") => filt(token, ")"),
            token if token.contains(",") => filt(token, ","),
            token if token.contains(";") => filt(token, ";"),
            _ => vec![token.to_string()],
        }
    }

    // ------------ CREATE QUERY -------------------------------

    fn handle_create_table_query(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<CreateParsedQuery, Box<dyn Error>>;

    fn handle_create_index_query(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<CreateParsedQuery, Box<dyn Error>>;

    fn handle_create_query(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<CreateParsedQuery, Box<dyn Error>>;

    // ------------ SELECT QUERY -------------------------------
    fn handle_select_query(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<SelectParsedQuery, Box<dyn Error>>;

    // ------------ WHERE CLAUSE -------------------------------
    fn parse_where_clause(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<Option<(String, RecordDataType)>, Box<dyn Error>>;
}

#[allow(private_bounds)]
pub trait QueryParser: QueryParserInner {
    fn get_parsed_query(&self, query: &str) -> Result<ParsedQueryResult, Box<dyn Error>>;
}

impl QueryParserInner for ParsedQueryResult {
    fn parse_where_clause(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<Option<(String, RecordDataType)>, Box<dyn Error>> {
        let where_pos = query_tokens
            .iter()
            .position(|token| token.to_lowercase() == "where");

        println!("\nwhere pos: {:?}\n", where_pos);
        if let Some(pos) = where_pos {
            let rest_tokens = query_tokens[pos..].to_vec();
            println!("rest tokens: {:?}\n", rest_tokens);

            if let Some(equality_pos) = rest_tokens.iter().position(|token| token.contains("=")) {
                if equality_pos >= 1 && rest_tokens.len() - 1 > equality_pos {
                    let mut field = String::new();
                    let mut val = String::new();

                    // (opening quote, closing quote) -> quote => double | single
                    let mut quotes_pos = vec![];
                    let mut quotes = String::new();

                    for (idx, token) in rest_tokens.iter().enumerate() {
                        if token.contains("\"") {
                            if quotes.is_empty() {
                                quotes = String::from("\"");
                            }
                            if quotes == "'" {
                                return Err(format!("Invalid quoting in where clause").into());
                            }
                            quotes_pos.push(idx);
                        } else if token.contains("'") {
                            if quotes.is_empty() {
                                quotes = String::from("'");
                            }
                            if quotes == "'" {
                                return Err(format!("Invalid quoting in where clause").into());
                            }
                            quotes_pos.push(idx);
                        }

                        // if quotes.is_empty() {
                        //     if token.contains("\"") {
                        //         quotes = String::from("\"");
                        //         quotes_pos.push(idx);
                        //     } else if token.contains("'") {
                        //         quotes = String::from("'");
                        //         quotes_pos.push(idx);
                        //     }
                        //     continue;
                        // } else if token.contains("\"") && quotes == "\"" {
                        //     quotes_pos.push(idx);
                        //     quotes = String::from("");
                        // } else if token.contains("'") && quotes == "'" {
                        //     quotes_pos.push(idx);
                        //     quotes = String::from("");
                        // } else {
                        //     return Err(format!("Invalid quoting in where clause").into());
                        // }
                    }

                    if rest_tokens[equality_pos].len() > 1 {
                        let equality_token_split = rest_tokens[equality_pos].split("=");
                        println!("equality split: {:?}", equality_token_split);
                    } else {
                        field = rest_tokens[equality_pos - 1].to_string();

                        val = rest_tokens[equality_pos + 1]
                            .replace("'", "")
                            .replace('"', "");
                    }

                    println!("Field: {}", field);
                    println!("Value: {}", val);

                    let record_value = RecordDataType::convert_str_to_recordformat(val.to_string());

                    return Ok(Some((field, record_value)));
                } else {
                    return Ok(None);
                }
            }
            return Ok(None);
        }

        Ok(None)
    }

    fn handle_select_query(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<SelectParsedQuery, Box<dyn Error>> {
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

        let mut tblname = query_tokens[from_pos.unwrap() + 1].to_owned();

        if tblname.contains(";") {
            tblname =
                self.filter_delim(query_tokens[from_pos.unwrap() + 1].to_owned())[0].to_string();
        }

        let where_clause = self.parse_where_clause(query_tokens);

        Ok(SelectParsedQuery {
            tblname,
            output_fields,
            where_clause,
        })
    }

    fn handle_create_table_query(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<CreateParsedQuery, Box<dyn Error>> {
        let mut tblname = query_tokens[2].to_owned();
        // If create has "IF" at idx 2 then there has to be IF NOT EXISTS which means
        // tblname is at 5th idx.
        if query_tokens[2] == "IF" {
            tblname = query_tokens[5].to_owned();
        }

        let cols = self.get_col_name(query_tokens)?;

        Ok(CreateParsedQuery {
            tblname,
            schematype: SchemaType::TABLE,
            cols,
        })
    }

    fn handle_create_index_query(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<CreateParsedQuery, Box<dyn Error>> {
        let on_clause_pos = query_tokens
            .iter()
            .position(|token| token.as_str() == "ON" || token.as_str() == "on");

        if on_clause_pos.is_none() {
            return Err(format!("bruh where is the ON clause?? you are suppose to have in the createindex query. Please add it.").into());
        };

        let mut tblname = query_tokens[on_clause_pos.unwrap() + 1].to_string();

        if tblname.contains("(") {
            tblname = self.filter_delim(tblname)[0].to_string();
        }

        let cols = self.get_col_name(query_tokens)?;

        Ok(CreateParsedQuery {
            tblname,
            schematype: SchemaType::INDEX,
            cols,
        })
    }

    fn handle_create_query(
        &self,
        query_tokens: Vec<String>,
    ) -> Result<CreateParsedQuery, Box<dyn Error>> {
        let schematype: SchemaType = SchemaType::try_from(query_tokens[1].to_owned())?;

        match schematype {
            SchemaType::TABLE => self.handle_create_table_query(query_tokens),
            SchemaType::INDEX => self.handle_create_index_query(query_tokens),
            _ => Err(format!("We don't support VIEWS and TRIGGERS query parsing as of now").into()),
        }
    }
}

impl QueryParser for ParsedQueryResult {
    fn get_parsed_query(&self, query: &str) -> Result<ParsedQueryResult, Box<dyn Error>> {
        let query_tokens: Vec<String> = query
            .split(' ')
            .map(|s| s.trim().to_string())
            .filter(|s| s != "")
            .collect::<Vec<String>>();

        let query_type = QueryType::from_str(&query_tokens[0].to_lowercase())?;

        match query_type {
            QueryType::CREATE => {
                let create_parsed_query = self.handle_create_query(query_tokens)?;
                Ok(ParsedQueryResult::CREATE(create_parsed_query))
            }
            QueryType::SELECT => {
                let select_parsed_query = self.handle_select_query(query_tokens)?;
                Ok(ParsedQueryResult::SELECT(select_parsed_query))
            }
            QueryType::INSERT => Err(format!("INSERT is not supported yet !!!").into()),
            QueryType::UPDATE => Err(format!("UPDATE is not supported yet !!!").into()),
            QueryType::DELETE => Err(format!("DELETE is not supported yet !!!").into()),
        }
    }
}
