use std::fmt;

#[derive(Debug)]
pub enum Error {
    Box(Box<dyn std::error::Error + Send + Sync>),
    Field(mssql_client::Error, &'static str),
    MssqlClient(mssql_client::Error),
    Str(&'static str),
    String(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Box(e) => e.fmt(f),
            Self::Field(e, field) => {
                e.fmt(f)?;
                f.write_str(" field: ")?;
                f.write_str(field)
            }
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
    #[inline]
    fn from(e: Box<E>) -> Self {
        Self::Box(e as _)
    }
}

impl From<mssql_client::Error> for Error {
    #[inline]
    fn from(e: mssql_client::Error) -> Self {
        Self::MssqlClient(e)
    }
}

impl From<&'static str> for Error {
    #[inline]
    fn from(e: &'static str) -> Self {
        Self::Str(e)
    }
}

impl From<String> for Error {
    #[inline]
    fn from(e: String) -> Self {
        Self::String(e)
    }
}
