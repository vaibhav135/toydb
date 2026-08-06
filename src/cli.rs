use std::{error::Error, path::Path};

use crate::{
    btree::Root, commands::Commands, file::Initialize, query::QueryExecutor, utils::readline,
};

#[derive(Debug)]
pub struct Cli {
    pub filename: Option<String>,
    pub cmd: Option<String>,
}

impl Cli {
    pub fn init(&self) -> Result<(), Box<dyn Error>> {
        let mut root = Root::default();
        if let Some(filename) = self.filename.to_owned() {
            if !Path::new(&filename).is_file() {
                return Err(format!("Invalid filepath !!!").into());
            }

            root.init(filename)?;
        }

        if let Some(cmd) = self.cmd.to_owned() {
            if self.filename.is_none() {
                return Err(format!(
                    "Must provide filepath, when executing a command beforehand !!!"
                )
                .into());
            }

            if !Commands::is_valid(&cmd) {
                return Err(format!("please provide a valid command").into());
            }

            Commands::process_cmd(&cmd, &root)?;
        }

        self.start(&mut root, self.filename.to_owned().unwrap())?;

        Ok(())
    }

    pub fn start(&self, root: &mut Root, filepath: String) -> Result<(), Box<dyn Error>> {
        loop {
            let mut input = readline()?;

            input = input.trim().to_owned();

            match input.as_str() {
                ".quit" | ".exit" => {
                    break;
                }
                _ => {
                    if Commands::is_valid(&input) {
                        Commands::process_cmd(&input, root)?;
                    } else {
                        let queryexec = QueryExecutor::new(input.to_string(), filepath.to_string());
                        queryexec.execute(root)?;
                    }
                }
            }
        }

        Ok(())
    }
}
