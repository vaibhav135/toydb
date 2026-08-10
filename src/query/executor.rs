use std::{
    error::Error,
    time::{Duration, Instant},
};

use crate::{
    btree::{Child, DBHeader, IndexRow, Root, Row, SchemaType, SqlSchema, TableRow},
    query::{
        common::{CreateParsedQuery, ParsedQueryResult, SelectParsedQuery},
        parser::QueryParser,
    },
    record_type::RecordDataType,
};

#[derive(Debug, PartialEq)]
pub enum QueryOperations {
    GetAll,
    SearchByID(u64),

    // In case of indexing the indexed val, will be the first one and then the row id will be the
    // second in interior or leaf index btree page.
    IdxSearchByVal(RecordDataType),

    // This is the case when the field is not indexed.
    // First is the col idx for which we are trying to find the data.
    // Second is the cold value we want to match.
    FullTableScanSearch(usize, RecordDataType),
    Empty,
}

#[derive(Debug)]
pub struct QueryExecutor {
    query: String,
    filepath: String,
}

impl QueryExecutor {
    pub fn new(query: String, filepath: String) -> Self {
        QueryExecutor { query, filepath }
    }

    pub fn execute(&self, root: &Root) -> Result<(), Box<dyn Error>> {
        let qstart_time = Instant::now();
        let qparser = ParsedQueryResult::default();
        let parsed_query = qparser.get_parsed_query(&self.query)?;
        let cur_table: &SqlSchema;

        let dbheader = &root.db_header;

        match parsed_query {
            ParsedQueryResult::SELECT(select_parsed_query) => {
                let schema_list = root.tables.get(&select_parsed_query.tblname);

                if schema_list.is_some() {
                    let tblpos = schema_list
                        .unwrap()
                        .iter()
                        .position(|tbl| tbl.schema_type == SchemaType::TABLE);

                    if tblpos.is_none() {
                        return Err(
                            format!("table not found!!! please type a valid table name").into()
                        );
                    }

                    let cols_to_print = &select_parsed_query.output_fields;
                    let tbl_schema = &schema_list.unwrap()[tblpos.unwrap()];

                    let childnode = Child::default();

                    let mut queryop = QueryOperations::GetAll;

                    // Set the query operation here for the table leaf to get the righ data.
                    self.handle_where_clause(
                        &childnode,
                        &tbl_schema,
                        &schema_list.unwrap(),
                        &select_parsed_query,
                        dbheader,
                        &qparser,
                        &mut queryop,
                    )?;

                    if let Row::TABLE(tablerow) =
                        &childnode.get_rows(&self.filepath, dbheader, &tbl_schema, queryop)?
                    {
                        let total_qexec_time = qstart_time.elapsed();

                        let orig_cols = self
                            .get_orig_cols(&qparser, tbl_schema.sql.to_string())?
                            .cols;

                        if cols_to_print.len() == 1 && cols_to_print[0] == "*" {
                            self.printrows(&orig_cols, total_qexec_time, tablerow, &orig_cols);
                        } else {
                            self.validate_output_fields(&cols_to_print, &orig_cols)?;

                            self.printrows(&cols_to_print, total_qexec_time, tablerow, &orig_cols);
                        }
                    }
                } else {
                    return Err(format!("table not found!!! please type a valid table name").into());
                }

                Ok(())
            }
            _ => Err(format!("Sorry we don't support any other query type for now").into()),
        }
    }

    fn handle_where_clause(
        &self,
        childnode: &Child,
        tbl_schema: &SqlSchema,
        schema_list: &Vec<SqlSchema>,
        select_parsed_query: &SelectParsedQuery,
        dbheader: &DBHeader,
        qparser: &ParsedQueryResult,
        queryop: &mut QueryOperations,
    ) -> Result<(), Box<dyn Error>> {
        if let Some((field, value)) = &select_parsed_query.where_clause {
            // let is_schema_idx =
            let create_index_schema_pos = schema_list
                .iter()
                .position(|tbl| tbl.schema_type == SchemaType::INDEX);

            let is_indexed = create_index_schema_pos.is_some();

            if is_indexed {
                // This means the schema is indexed.
                let idxrows = self.get_indexed_rows(
                    create_index_schema_pos.unwrap(),
                    childnode,
                    schema_list,
                    dbheader,
                    qparser,
                    field,
                    value,
                )?;

                if idxrows.total_rows > 0 {
                    *queryop = QueryOperations::SearchByID(idxrows.rows[0]);
                } else {
                    *queryop = QueryOperations::Empty;
                }
            } else {
                // Incase the fields are not indexed
                let (col_idx, seek) =
                    self.get_idx_and_val_for_fss(tbl_schema, qparser, field, value)?;

                *queryop = QueryOperations::FullTableScanSearch(col_idx, seek);
            }
        }

        Ok(())
    }

    // fss = full scan search
    fn get_idx_and_val_for_fss(
        &self,
        tblschema: &SqlSchema,
        qparser: &ParsedQueryResult,
        where_field: &String,
        where_value: &RecordDataType,
    ) -> Result<(usize, RecordDataType), Box<dyn Error>> {
        // This is the create table query.
        let orig_query = self.get_orig_cols(&qparser, tblschema.sql.to_string())?;
        let field_pos = orig_query.cols.iter().position(|col| col == where_field);

        if field_pos.is_some() {
            return Ok((field_pos.unwrap(), where_value.to_owned()));
        } else {
            return Err(format!(
                "Invalid field: this field isn't the part of the table you are querying for"
            )
            .into());
        }
    }

    fn get_indexed_rows(
        &self,
        create_idx_schema_pos: usize,
        childnode: &Child,
        schema_list: &Vec<SqlSchema>,
        dbheader: &DBHeader,
        qparser: &ParsedQueryResult,
        where_field: &String,
        where_value: &RecordDataType,
    ) -> Result<IndexRow, Box<dyn Error>> {
        let mut idxrows = IndexRow::default();
        let idx_schema = &schema_list[create_idx_schema_pos];

        let create_idx_query = self.get_orig_cols(&qparser, idx_schema.sql.to_string())?;

        // Check if the field, for which we are doing the scan is valid.
        if create_idx_query.cols.contains(&where_field) {
            if let Row::INDEX(idx_row) = childnode.get_rows(
                &self.filepath,
                dbheader,
                &idx_schema,
                QueryOperations::IdxSearchByVal(where_value.to_owned()),
            )? {
                idxrows = idx_row;
            }
        } else {
            return Err(format!(
                "Invalid field: this field isn't the part of the table you are querying for"
            )
            .into());
        }

        Ok(idxrows)
    }

    fn get_orig_cols(
        &self,
        qparser: &ParsedQueryResult,
        sql: String,
    ) -> Result<CreateParsedQuery, Box<dyn Error>> {
        // We need to get the original cols that were there in create query. If the user
        // requested fields don't match with original cols then we raise Err.
        let ParsedQueryResult::CREATE(create_parsed_query_res) = qparser.get_parsed_query(&sql)?
        else {
            return Err(format!("").into());
        };

        Ok(create_parsed_query_res)
    }

    fn validate_output_fields(
        &self,
        fields: &Vec<String>,

        orig_cols: &Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        if !fields.iter().all(|f| orig_cols.contains(f)) {
            return Err(format!("Invalid cols found in the select query. Please recheck your table and and request for existing cols").into());
        }

        Ok(())
    }

    fn printrows(
        &self,
        user_output_cols: &Vec<String>,
        qexec_time: Duration,
        tablerow: &TableRow,
        orig_cols: &Vec<String>,
    ) {
        // This will fix the order of the user output cols. Basically match the order with original
        // col. And second create the vec of the same length with empty string (means these are the
        // elements which are not requested by the user). This was we can only print the cols which
        // are requested by the user. Because remember the order of the value i.e row follow the
        // order of the original create query, and the user might not write the select question
        // output field in the same order or even the no. of col might differ.
        let cols_to_print = orig_cols
            .iter()
            .map(|col| {
                if user_output_cols.contains(col) {
                    col.to_string()
                } else {
                    String::from("")
                }
            })
            .collect::<Vec<String>>();

        println!("\n\n");
        for field in user_output_cols {
            print!("{field} | ");
        }
        println!("\n");

        for (_rowid, row) in &tablerow.rows {
            for (idx, field) in row.iter().enumerate() {
                if !cols_to_print[idx].is_empty() {
                    match field {
                        RecordDataType::STR(val) => {
                            print!("{}", val);
                        }
                        RecordDataType::INT8(val) => {
                            print!("{}", val);
                        }
                        RecordDataType::INT32(val) => {
                            print!("{}", val);
                        }
                        RecordDataType::INT16(val) => {
                            print!("{}", val);
                        }
                        RecordDataType::INT64(val) => {
                            print!("{}", val);
                        }
                        RecordDataType::FLOAT(val) => {
                            print!("{}", val);
                        }
                        RecordDataType::BLOB(val) => {
                            print!("{:?}", val);
                        }
                        RecordDataType::NULL => {
                            print!("");
                        }
                    }
                    if idx < row.len() - 1 {
                        print!(" | ");
                    }
                }
            }
            println!("\n");
        }

        println!("Total rows: {}", tablerow.total_rows);
        println!("Total execution time: {:?}\n\n", qexec_time);
    }
}
