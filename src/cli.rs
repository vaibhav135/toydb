use std::{env::current_dir, error::Error, path::Path};

use crate::{btree::Root, file::Initialize, utils::readline};

#[derive(Debug)]
pub struct Cli {
    pub filename: Option<String>,
    pub cmd: Option<String>,
}

impl Cli {
    pub fn init(&self) -> Result<(), Box<dyn Error>> {
        let mut root = Root::default();
        if let Some(filename) = self.filename.to_owned() {
            if !Path::new(&filename).is_dir() {
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
        }

        self.start(&mut root)?;

        Ok(())
    }

    pub fn start(&self, root: &mut Root) -> Result<(), Box<dyn Error>> {
        loop {
            let mut input = readline()?;

            input = input.trim().to_owned();

            match input.as_str() {
                ".quit" | ".exit" => {
                    break;
                }
                _ => {
                    println!("Invalid Command!!!")
                }
            }
        }

        Ok(())
    }
}
