wit_bindgen_rust::import!("../../wit/ephemeral/sqlite.wit");

use sqlite::Connection as RawConnection;

/// Errors which may be raised by the methods of `Store`
pub type Error = sqlite::Error;

///
pub type DataTypeParam<'a> = sqlite::ValueParam<'a>;
///
pub type DataTypeResult = sqlite::ValueResult;

/// Represents a store in which key value tuples may be placed
#[derive(Debug)]
pub struct Connection(RawConnection);

impl Connection {
    /// Open a connection
    pub fn open(database: &str) -> Result<Self, Error> {
        Ok(Self(sqlite::open(database)?))
    }

    /// Execute a statement against the database
    pub fn execute(
        &self,
        statement: &str,
        parameters: &[sqlite::ValueParam<'_>],
    ) -> Result<(), Error> {
        sqlite::execute(self.0, statement, parameters)?;
        Ok(())
    }

    /// Make a query against the database
    pub fn query(
        &self,
        query: &str,
        parameters: &[DataTypeParam<'_>],
    ) -> Result<sqlite::QueryResult, Error> {
        sqlite::query(self.0, query, parameters)
    }
}

impl sqlite::QueryResult {
    /// Get all the rows for this query result
    pub fn rows<'a>(&'a self) -> impl Iterator<Item = Row<'a>> {
        self.rows.iter().map(|r| Row {
            columns: self.columns.as_slice(),
            result: r,
        })
    }
}

/// A database row result
pub struct Row<'a> {
    columns: &'a [String],
    result: &'a sqlite::RowResult,
}

impl<'a> Row<'a> {
    /// Get a value by its column name
    pub fn get<T: TryFrom<&'a sqlite::ValueResult>>(&self, column: &str) -> Option<T> {
        let i = self.columns.iter().position(|c| c == column)?;
        self.result.get(i)
    }
}

impl sqlite::RowResult {
    pub fn get<'a, T: TryFrom<&'a sqlite::ValueResult>>(&'a self, index: usize) -> Option<T> {
        self.values.get(index).and_then(|c| c.try_into().ok())
    }
}

impl<'a> TryFrom<&'a sqlite::ValueResult> for bool {
    type Error = ();

    fn try_from(value: &'a sqlite::ValueResult) -> Result<Self, Self::Error> {
        match value {
            sqlite::ValueResult::Integer(i) => Ok(*i != 0),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a sqlite::ValueResult> for u32 {
    type Error = ();

    fn try_from(value: &'a sqlite::ValueResult) -> Result<Self, Self::Error> {
        match value {
            sqlite::ValueResult::Integer(i) => Ok(*i as u32),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a sqlite::ValueResult> for &'a str {
    type Error = ();

    fn try_from(value: &'a sqlite::ValueResult) -> Result<Self, Self::Error> {
        match value {
            sqlite::ValueResult::Text(s) => Ok(s.as_str()),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
