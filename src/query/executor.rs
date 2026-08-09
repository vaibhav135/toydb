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

#[derive(Debug)]
pub enum QueryOperations {
    GetAll,
    SearchByID(u64),

    // In case of indexing the indexed val, will be the first one and then the row id will be the
    // second in interior or leaf index btree page.
    IdxSearchByVal(RecordDataType),
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

                    let idxpos = schema_list
                        .unwrap()
                        .iter()
                        .position(|tbl| tbl.schema_type == SchemaType::INDEX);

                    let select_fields = &select_parsed_query.output_fields;
                    let cur_table = &schema_list.unwrap()[tblpos.unwrap()];

                    let childnode = Child::default();

                    let idxrows = self.get_idx_rows(
                        &childnode,
                        idxpos,
                        &schema_list.unwrap(),
                        &select_parsed_query,
                        dbheader,
                        &qparser,
                    )?;

                    let mut tbl_queryop = QueryOperations::GetAll;

                    if idxrows.total_rows > 0 {
                        tbl_queryop = QueryOperations::SearchByID(idxrows.rows[0]);
                    }

                    if let Row::TABLE(tablerow) =
                        &childnode.get_rows(&self.filepath, dbheader, &cur_table, tbl_queryop)?
                    {
                        let total_qexec_time = qstart_time.elapsed();

                        let orig_cols = self
                            .get_orig_cols(&qparser, cur_table.sql.to_string())?
                            .cols;

                        if select_fields.len() == 1 && select_fields[0] == "*" {
                            self.printrows(&orig_cols, total_qexec_time, tablerow, &orig_cols);
                        } else {
                            self.validate_output_fields(&select_fields, &orig_cols)?;

                            self.printrows(&select_fields, total_qexec_time, tablerow, &orig_cols);
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

    fn get_idx_rows(
        &self,
        childnode: &Child,
        idxpos: Option<usize>,
        schema_list: &Vec<SqlSchema>,
        select_parsed_query: &SelectParsedQuery,
        dbheader: &DBHeader,
        qparser: &ParsedQueryResult,
    ) -> Result<IndexRow, Box<dyn Error>> {
        let mut idxrows = IndexRow::default();

        if let Some((field, value)) = &select_parsed_query.where_clause {
            if let Some(pos) = idxpos {
                let idx_schema = &schema_list[pos];

                let idx_query = self.get_orig_cols(&qparser, idx_schema.sql.to_string())?;

                if idx_query.cols.contains(&field) {
                    if let Row::INDEX(idx_row) = childnode.get_rows(
                        &self.filepath,
                        dbheader,
                        &idx_schema,
                        QueryOperations::IdxSearchByVal(value.to_owned()),
                    )? {
                        idxrows = idx_row;
                    }
                }
            }
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
            println!("\n\n");
        }

        println!("Total rows: {}", tablerow.total_rows);
        println!("Total execution time: {:?}", qexec_time);
    }
}
