use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum CustomError {
    InvalidOffsetValueError,
}

impl Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CustomError::InvalidOffsetValueError => write!(f, "Invalid offset value found"),
        }
    }
}

impl Error for CustomError {}
