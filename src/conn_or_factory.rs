use mssql_client::{Connection, ConnectionFactory};

pub enum ConnOrFactory {
    Connection(Connection),
    Factory(ConnectionFactory),
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
