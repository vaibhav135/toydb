use std::str::FromStr;

use crate::btree::SchemaType;

/**
* NOTE: [About type affinity]:
* one interesting thing about sqlite is that it supports - type affinity by default.
* Meaning any of the fields can be of a storage class (i.e, affinity), but it's only
* a recommended type and not required (i.e, not enforced).
* If you want strict types then you have to create table with the strict table option.
*/
#[derive(Debug)]
pub enum ColType {
    TEXT,
    INTEGER,
    NUMERIC,
    REAL,
    BLOB,
}

#[derive(Debug, Default)]
pub enum QueryType {
    #[default]
    SELECT,

    CREATE,
    INSERT,
    UPDATE,
    DELETE,
}

#[derive(Debug, Default)]
pub struct SelectParsedQuery {
    pub tblname: String,
    pub output_fields: Vec<String>,
}

#[derive(Debug, Default)]
pub struct CreateParsedQuery {
    pub tblname: String,
    pub schematype: SchemaType,

    // NOTE:
    // Currently we'll only have support for name.
    // I did try to add checks for constraint and types and whatnot
    // but the grammer is too big to handle (too many cases) for my
    // super-dumb parser to handle. Maybe once I learn more about AST's
    // parser generator (how lemon parser generator works) since that's what
    // sqlite does. Then maybe we'll give it a real shot on handling all
    // that neat stuff. For now we will only extract col names that's it.
    pub cols: Vec<String>,
}

#[derive(Debug)]
pub enum ParsedQueryResult {
    CREATE(CreateParsedQuery),
    SELECT(SelectParsedQuery),
}

impl Default for ParsedQueryResult {
    fn default() -> Self {
        ParsedQueryResult::SELECT(SelectParsedQuery::default())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryClause {
    From,
    Where,
    // NOTE: Order and By (similarly Group and By) are two different
    // clause but I am writing it as one for now. Since we are making
    // a super simple (dumb) parser.
    GroupBy,
    OrderBy,
    Limit,
    Offset,
}

impl FromStr for QueryType {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "create" => Ok(QueryType::CREATE),
            "update" => Ok(QueryType::UPDATE),
            "delete" => Ok(QueryType::DELETE),
            "select" => Ok(QueryType::SELECT),
            "insert" => Ok(QueryType::INSERT),
            _ => Err(
                "invalid query type. Check your spelling or make sure it's one of these SELECT, CREATE, UPDATE, DELETE (valid in lower case too)",
            ),
        }
    }
}

impl TryFrom<String> for QueryClause {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "from" => Ok(QueryClause::From),
            "where" => Ok(QueryClause::Where),
            "group by" => Ok(QueryClause::GroupBy),
            "order by" => Ok(QueryClause::OrderBy),
            "limit" => Ok(QueryClause::Limit),
            "offset" => Ok(QueryClause::Offset),
            _ => Err(
                "invalid query clause. Check your spelling or make sure it's one of these FROM, WHERE, GROUP BY, ORDER BY, LIMIT, OFFSET (valid in lower case too)",
            ),
        }
    }
}

impl From<QueryClause> for String {
    fn from(value: QueryClause) -> Self {
        match value {
            QueryClause::From => String::from("from"),
            QueryClause::Where => String::from("where"),
            QueryClause::OrderBy => String::from("order by"),
            QueryClause::GroupBy => String::from("group by"),
            QueryClause::Limit => String::from("limit"),
            QueryClause::Offset => String::from("offset"),
        }
    }
}
