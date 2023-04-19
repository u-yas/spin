mod host_component;

use std::collections::HashMap;

use rand::Rng;
use spin_core::{
    async_trait,
    sqlite::{self, Host},
};

pub use host_component::DatabaseLocation;
pub use host_component::SqliteComponent;

pub struct SqliteImpl {
    location: DatabaseLocation,
    connections: HashMap<sqlite::Connection, rusqlite::Connection>,
}

impl SqliteImpl {
    pub fn new(location: DatabaseLocation) -> Self {
        Self {
            location,
            connections: HashMap::default(),
        }
    }

    pub fn component_init(&mut self) {}
}

#[async_trait]
impl Host for SqliteImpl {
    async fn open(
        &mut self,
        _database: String,
    ) -> anyhow::Result<Result<spin_core::sqlite::Connection, spin_core::sqlite::Error>> {
        // TODO: handle more than one database
        let conn = match &self.location {
            DatabaseLocation::InMemory => rusqlite::Connection::open_in_memory()?,
            DatabaseLocation::Path(p) => rusqlite::Connection::open(p)?,
        };

        // TODO: this is not the best way to do this...
        let mut rng = rand::thread_rng();
        let c: sqlite::Connection = rng.gen();
        self.connections.insert(c, conn);
        Ok(Ok(c))
    }

    async fn execute(
        &mut self,
        connection: sqlite::Connection,
        statement: String,
        parameters: Vec<sqlite::Value>,
    ) -> anyhow::Result<Result<(), sqlite::Error>> {
        let c = self.connections.get(&connection).expect("TODO");
        let mut s = c.prepare_cached(&statement).expect("TODO");
        s.execute(rusqlite::params_from_iter(convert_data(
            parameters.into_iter(),
        )))
        .map_err(|e| sqlite::Error::Io(e.to_string()))?;
        Ok(Ok(()))
    }

    async fn query(
        &mut self,
        connection: sqlite::Connection,
        query: String,
        parameters: Vec<sqlite::Value>,
    ) -> anyhow::Result<Result<Vec<sqlite::Row>, sqlite::Error>> {
        let c = self.connections.get(&connection).expect("TODO");
        let mut statement = c
            .prepare_cached(&query)
            .map_err(|e| sqlite::Error::Io(e.to_string()))?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(convert_data(parameters.into_iter())),
                |row| {
                    let mut values = vec![];
                    for column in 0.. {
                        let name = match row.as_ref().column_name(column) {
                            Ok(n) => n.to_owned(),
                            Err(rusqlite::Error::InvalidColumnIndex(_)) => break,
                            Err(_) => todo!(),
                        };
                        let value = match row.get_ref(column) {
                            Ok(rusqlite::types::ValueRef::Null) => sqlite::Value::Null,
                            Ok(rusqlite::types::ValueRef::Integer(i)) => sqlite::Value::Integer(i),
                            Ok(rusqlite::types::ValueRef::Real(f)) => sqlite::Value::Real(f),
                            Ok(rusqlite::types::ValueRef::Text(t)) => {
                                sqlite::Value::Text(String::from_utf8(t.to_vec()).unwrap())
                            }
                            Ok(rusqlite::types::ValueRef::Blob(b)) => {
                                sqlite::Value::Blob(b.to_vec())
                            }
                            Err(rusqlite::Error::InvalidColumnIndex(_)) => break,
                            _ => todo!(),
                        };
                        values.push(sqlite::ColumnValue { name, value });
                    }
                    Ok(sqlite::Row { values })
                },
            )
            .map_err(|e| sqlite::Error::Io(e.to_string()))?;
        Ok(Ok(rows.collect::<Result<_, _>>()?))
    }

    async fn close(&mut self, connection: spin_core::sqlite::Connection) -> anyhow::Result<()> {
        let _ = self.connections.remove(&connection);
        Ok(())
    }
}

fn convert_data(
    arguments: impl Iterator<Item = sqlite::Value>,
) -> impl Iterator<Item = rusqlite::types::Value> {
    arguments.map(|a| match a {
        sqlite::Value::Null => rusqlite::types::Value::Null,
        sqlite::Value::Integer(i) => rusqlite::types::Value::Integer(i),
        sqlite::Value::Real(r) => rusqlite::types::Value::Real(r),
        sqlite::Value::Text(t) => rusqlite::types::Value::Text(t),
        sqlite::Value::Blob(b) => rusqlite::types::Value::Blob(b),
    })
}
