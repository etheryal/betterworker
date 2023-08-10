use std::convert::TryFrom;
use std::pin::Pin;

use betterworker_sys::{
    D1Database as D1DatabaseSys, D1PreparedStatement as D1PreparedStatementSys,
};
use futures_util::Future;
use js_sys::{Array, ArrayBuffer, Object, Uint8Array};
use send_wrapper::SendWrapper;
use serde::Deserialize;
pub use serde_wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::error::Error;
use crate::result::Result;

pub mod macros;

/// A D1 Database.
pub struct Database(SendWrapper<D1DatabaseSys>);

impl Database {
    /// Prepare a query statement from a query string.
    pub fn prepare<T: Into<String>>(&self, query: T) -> PreparedStatement {
        self.0.prepare(&query.into()).into()
    }

    /// Dump the data in the database to a `Vec`.
    pub async fn dump(&self) -> Result<Vec<u8>> {
        let future = SendWrapper::new(JsFuture::from(self.0.dump()));
        wrap_send(async move {
            let array_buffer = future.await?;
            let array_buffer = array_buffer.dyn_into::<ArrayBuffer>()?;
            let array = Uint8Array::new(&array_buffer);
            let mut vec = Vec::with_capacity(array.length() as usize);
            array.copy_to(&mut vec);
            Ok(vec)
        })
        .await
    }

    /// Batch execute one or more statements against the database.
    ///
    /// Returns the results in the same order as the provided statements.
    pub async fn batch<T>(&self, statements: Vec<PreparedStatement>) -> Result<Vec<D1Result<T>>>
    where
        T: for<'a> Deserialize<'a>, {
        let future = {
            let statements = statements
                .into_iter()
                .map(|s| s.0.take())
                .collect::<Array>();
            SendWrapper::new(JsFuture::from(self.0.batch(statements)))
        };
        wrap_send(async move {
            let results = future.await?;
            let results = results.dyn_into::<Array>()?;
            let mut vec = Vec::with_capacity(results.length() as usize);
            for value in results.iter() {
                let result = serde_wasm_bindgen::from_value(value)?;
                vec.push(D1Result::from(result));
            }
            Ok(vec)
        })
        .await
    }

    /// Execute one or more queries directly against the database.
    ///
    /// The input can be one or multiple queries separated by `\n`.
    ///
    /// # Considerations
    ///
    /// This method can have poorer performance (prepared statements can be
    /// reused in some cases) and, more importantly, is less safe. Only use
    /// this method for maintenance and one-shot tasks (example: migration
    /// jobs).
    ///
    /// If an error occurs, an exception is thrown with the query and error
    /// messages, execution stops and further statements are not executed.
    pub async fn exec(&self, query: &str) -> Result<D1ExecResult> {
        let future = SendWrapper::new(JsFuture::from(self.0.exec(query)));
        wrap_send(async move {
            let result = serde_wasm_bindgen::from_value(future.await?)?;
            Ok(D1ExecResult::from(result))
        })
        .await
    }
}

impl AsRef<JsValue> for Database {
    fn as_ref(&self) -> &JsValue {
        &self.0
    }
}

impl From<D1DatabaseSys> for Database {
    fn from(inner: D1DatabaseSys) -> Self {
        Self(SendWrapper::new(inner))
    }
}

impl TryFrom<Object> for Database {
    type Error = Error;

    fn try_from(obj: Object) -> Result<Self> {
        const TYPE_NAME: &'static str = "D1Database";

        let data = if obj.constructor().name() == TYPE_NAME {
            obj.unchecked_into()
        } else {
            return Err(Error::BindingCast(
                TYPE_NAME.to_string(),
                obj.constructor().name().as_string().unwrap(),
            ));
        };
        Ok(Self(SendWrapper::new(data)))
    }
}

impl From<Database> for JsValue {
    fn from(ns: Database) -> Self {
        JsValue::from(ns.0.take())
    }
}

// A D1 prepared query statement.
pub struct PreparedStatement(SendWrapper<D1PreparedStatementSys>);

impl PreparedStatement {
    /// Bind one parameter to the statement.
    /// Consumes the old statement and returns a new statement with the bound
    /// parameters.
    ///
    /// D1 follows the SQLite convention for prepared statements parameter
    /// binding.
    ///
    /// # Considerations
    ///
    /// Supports Ordered (?NNNN) and Anonymous (?) parameters - named parameters
    /// are currently not supported.
    pub fn bind(self, value: impl Into<JsValue>) -> Result<Self> {
        self.bind_many(&[value.into()])
    }

    /// Bind one or more parameters to the statement.
    /// Consumes the old statement and returns a new statement with the bound
    /// parameters.
    ///
    /// D1 follows the SQLite convention for prepared statements parameter
    /// binding.
    ///
    /// # Considerations
    ///
    /// Supports Ordered (?NNNN) and Anonymous (?) parameters - named parameters
    /// are currently not supported.
    pub fn bind_many(self, values: &[JsValue]) -> Result<Self> {
        let array: Array = values.iter().collect::<Array>();

        match self.0.bind(array) {
            Ok(stmt) => Ok(PreparedStatement::from(stmt)),
            Err(err) => Err(Error::from(err)),
        }
    }

    /// Return the first row of results.
    ///
    /// If `col_name` is `Some`, returns that single value, otherwise returns
    /// the entire object.
    ///
    /// If the query returns no rows, then this will return `None`.
    ///
    /// If the query returns rows, but column does not exist, then this will
    /// return an `Err`.
    pub async fn first<T>(&self, col_name: Option<&str>) -> Result<Option<T>>
    where
        T: for<'a> Deserialize<'a>, {
        let future = SendWrapper::new(JsFuture::from(self.0.first(col_name)));
        wrap_send(async move {
            let js_value = future.await?;
            let value = serde_wasm_bindgen::from_value(js_value)?;
            Ok(value)
        })
        .await
    }

    /// Executes a query against the database but only return metadata.
    pub async fn run<T>(&self) -> Result<D1Result<T>>
    where
        T: for<'a> Deserialize<'a>, {
        let future = SendWrapper::new(JsFuture::from(self.0.run()));
        wrap_send(async move {
            let result = serde_wasm_bindgen::from_value(future.await?)?;
            Ok(D1Result::from(result))
        })
        .await
    }

    /// Executes a query against the database and returns all rows and metadata.
    pub async fn all<T>(&self) -> Result<D1Result<T>>
    where
        T: for<'a> Deserialize<'a>, {
        let future = SendWrapper::new(JsFuture::from(self.0.all()));
        wrap_send(async move {
            let result = serde_wasm_bindgen::from_value(future.await?)?;
            Ok(D1Result::from(result))
        })
        .await
    }

    /// Executes a query against the database and returns a `Vec` of rows
    /// instead of objects.
    pub async fn raw<T>(&self) -> Result<Vec<Vec<T>>>
    where
        T: for<'a> Deserialize<'a>, {
        let future = SendWrapper::new(JsFuture::from(self.0.raw()));
        wrap_send(async move {
            let result = future.await?;
            let result = result.dyn_into::<Array>()?;
            let mut vec = Vec::with_capacity(result.length() as usize);
            for value in result.iter() {
                let value = serde_wasm_bindgen::from_value(value)?;
                vec.push(value);
            }
            Ok(vec)
        })
        .await
    }
}

impl From<D1PreparedStatementSys> for PreparedStatement {
    fn from(inner: D1PreparedStatementSys) -> Self {
        Self(SendWrapper::new(inner))
    }
}

/// The result of a D1 query execution.
#[derive(Deserialize)]
pub struct D1Result<T> {
    results: Vec<T>,
    success: bool,
    error: Option<String>,
}

impl<T> D1Result<T> {
    /// Returns `true` if the result indicates a success, otherwise `false`.
    pub fn success(&self) -> bool {
        self.success
    }

    /// Return the error contained in this result.
    ///
    /// Returns `None` if the result indicates a success.
    pub fn error(&self) -> Option<&String> {
        self.error.as_ref()
    }

    /// Retrieve the collection of result objects, or an `Err` if an error
    /// occurred.
    pub fn results(&self) -> &Vec<T> {
        &self.results
    }

    /// Consumes the result and returns the collection of result objects.
    pub fn into_results(self) -> Vec<T> {
        self.results
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

fn wrap_send<Fut, O>(f: Fut) -> Pin<Box<dyn Future<Output = O> + Send + Sync + 'static>>
where
    Fut: Future<Output = O> + 'static, {
    Box::pin(SendWrapper::new(f))
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use crate::test_assertions::*;

    use super::*;

    assert_impl_all!(Database: Send, Sync);
    assert_impl_all!(PreparedStatement: Send, Sync);
    assert_impl_all!(D1Result<String>: Send, Sync);

    async_assert_fn!(Database::dump(_): Send & Sync);
    async_assert_fn!(Database::batch<String>(_, _): Send & Sync);
    async_assert_fn!(Database::exec(_, _): Send & Sync);

    async_assert_fn!(PreparedStatement::bind(_, String): Send & Sync);
    async_assert_fn!(PreparedStatement::bind_many(_, _): Send & Sync);
    async_assert_fn!(PreparedStatement::first<String>(_, _): Send & Sync);
    async_assert_fn!(PreparedStatement::run<String>(_): Send & Sync);
    async_assert_fn!(PreparedStatement::all<String>(_): Send & Sync);
    async_assert_fn!(PreparedStatement::raw<String>(_): Send & Sync);
}
