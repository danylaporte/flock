use std::fmt;

#[derive(Debug)]
pub enum Error {
    Box(Box<dyn std::error::Error + Send + Sync>),
    MssqlClient(mssql_client::Error),
    Str(&'static str),
    String(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Box(e) => e.fmt(f),
            Self::MssqlClient(e) => e.fmt(f),
            Self::Str(s) => f.write_str(s),
            Self::String(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}

impl<E> From<Box<E>> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(e: Box<E>) -> Self {
        Self::Box(e as _)
    }
}

impl From<mssql_client::Error> for Error {
    fn from(e: mssql_client::Error) -> Self {
        Self::MssqlClient(e)
    }
}

impl From<&'static str> for Error {
    fn from(e: &'static str) -> Self {
        Self::Str(e)
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Self::String(e)
    }
}