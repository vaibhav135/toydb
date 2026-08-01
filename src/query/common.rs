use std::str::FromStr;

#[derive(Debug, Default)]
pub enum QueryType {
    CREATE,

    #[default]
    SELECT,
    UPDATE,
    DELETE,
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

// parse the query and return tokenized strings.
/*
 *
 * -----------------Query Parser -----------------------------------------
 *
 * enum QueryType {
 *     Create,
 *     Select,
 *     Update,
 *     Delete,
 * }
 *
 * ParsedQuery {
 *   tablename:
 *   output_fields: * or specific fields
 *   QueryType
 *   WHERE CLAUSE option<>
 *   LIMIT option<>
 * }
 *
 * implment from for this QueryType
 *
 *
 *
 *
 * tokenized string
 *   select * from xyz; [ full table scan]
 *
 *   ["select", "*", "from", "xyz"]
 *
 *   query_type: QueryType = tokenized_query[0].into();
 *
 *   ouput_fields = ["*"];
 *
 *
 *   if curr_elem == "from" {
 *      tablename = tokenized_query[idx+1];
 *   }
 *
 *
 *
 * ----------------------- Executor Take over -------------------------------
 *
 *
 * impl QueryExecutor {
 *       fn execute(&self, query: String) {
 *           parse(query)
 *       }
 * }
 *
 *   if exists root.sqlschema[tablename] {
 *      let table = root.sqlschema[tablenmae];
 *      start_pg = table.rootpg;
 *
 *      read buffer
 *
 *   }else {
 *       Raise Error("Invalid fields !!!")
 *   }
 *
 *
 *
 * */
