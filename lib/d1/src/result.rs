use serde::Deserialize;

use crate::error::DatabaseError;

pub type Result<T = ()> = std::result::Result<T, DatabaseError>;

/// The metadata of a D1 database execution.
#[derive(Deserialize)]
pub struct D1ResultMetadata {
    #[serde(default)]
    duration: u64,

    #[serde(default)]
    rows_read: u64,
    
    #[serde(default)]
    rows_written: u64,
}

impl D1ResultMetadata {
    /// Returns the amount of time it took to execute the query in milliseconds.
    pub fn duration(&self) -> u64 {
        self.duration
    }

    /// Returns the number of rows read (scanned) by this query
    pub fn rows_read(&self) -> u64 {
        self.rows_read
    }

    /// Returns the number of rows written by this query
    pub fn rows_written(&self) -> u64 {
        self.rows_written
    }
}

#[derive(Deserialize)]
/// The result of a D1 database execution.
pub struct D1Result<T = ()> {
    #[serde(default = "Vec::new")]
    results: Vec<T>,
    success: bool,
    meta: Option<D1ResultMetadata>,
}

impl<T> D1Result<T> {
    /// Returns a reference to the results of the query execution.
    pub fn results(&self) -> &Vec<T> {
        &self.results
    }

    /// Takes the results of the query execution.
    pub fn take_results(self) -> Vec<T> {
        self.results
    }

    /// Returns the metadata of the query execution.
    pub fn meta(&self) -> Option<&D1ResultMetadata> {
        self.meta.as_ref()
    }

    /// Returns whether the query was successful.
    pub fn success(&self) -> bool {
        self.success
    }
}

/// The result of a single D1 database execution.
#[derive(Deserialize)]
pub struct D1ExecResult {
    count: Option<u32>,
    duration: Option<f64>,
}

impl D1ExecResult {
    /// Returns the amount of rows affected by the query.
    pub fn count(&self) -> Option<u32> {
        self.count
    }

    /// Returns the amount of time it took to execute the query.
    pub fn duration(&self) -> Option<f64> {
        self.duration
    }
}
