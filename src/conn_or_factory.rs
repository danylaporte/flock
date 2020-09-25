use crate::Result;
use mssql_client::{Connection, ConnectionFactory};

pub enum ConnOrFactory {
    Connection(Connection),
    Factory(ConnectionFactory),
}

impl ConnOrFactory {
    pub async fn connect(self) -> Result<Connection> {
        Ok(match self {
            ConnOrFactory::Connection(conn) => conn,
            ConnOrFactory::Factory(fact) => fact.create_connection().await?,
        })
    }

    pub fn from_env(s: &str) -> Result<ConnOrFactory> {
        Ok(ConnOrFactory::Factory(ConnectionFactory::from_env(s)?))
    }
}

impl From<Connection> for ConnOrFactory {
    fn from(c: Connection) -> Self {
        ConnOrFactory::Connection(c)
    }
}

impl From<ConnectionFactory> for ConnOrFactory {
    fn from(f: ConnectionFactory) -> Self {
        ConnOrFactory::Factory(f)
    }
}
