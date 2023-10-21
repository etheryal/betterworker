use std::convert::TryFrom;
use std::pin::Pin;

use betterworker_sys::{
    D1Database as D1DatabaseSys, D1PreparedStatement as D1PreparedStatementSys,
};
use futures_util::Future;
use js_sys::{Array, ArrayBuffer, Object, Uint8Array};
use result::D1ExecResult;
use send_wrapper::SendWrapper;
use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::error::DatabaseError;
use crate::result::{D1Result, Result};

pub mod error;
pub mod macros;
pub mod result;

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
            let array_buffer = future.await.map_err(map_promise_err)?;
            let array_buffer = array_buffer
                .dyn_into::<ArrayBuffer>()
                .map_err(|_| DatabaseError::JsCast)?;
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
            let results = future.await.map_err(map_promise_err)?;
            let results = results
                .dyn_into::<Array>()
                .map_err(|_| DatabaseError::JsCast)?;
            let mut vec = Vec::with_capacity(results.length() as usize);
            for value in results.iter() {
                let result = serde_wasm_bindgen::from_value(value)?;
                vec.push(result);
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
            let value = future.await.map_err(map_promise_err)?;
            let result = serde_wasm_bindgen::from_value(value)?;
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
    type Error = DatabaseError;

    fn try_from(obj: Object) -> Result<Self> {
        const TYPE_NAME: &'static str = "D1Database";

        let data = if obj.constructor().name() == TYPE_NAME {
            obj.unchecked_into()
        } else {
            return Err(DatabaseError::InvalidBinding);
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

        self.0
            .bind(array)
            .map(PreparedStatement::from)
            .map_err(|err| {
                let reason = err.as_string().unwrap_or("Unkown reason".to_string());
                DatabaseError::BindParameter(reason)
            })
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
            let js_value = future.await.map_err(map_promise_err)?;
            let value = serde_wasm_bindgen::from_value(js_value)?;
            Ok(value)
        })
        .await
    }

    /// Executes a query against the database but only return metadata.
    pub async fn run(&self) -> Result<D1Result> {
        let future = SendWrapper::new(JsFuture::from(self.0.run()));
        wrap_send(async move {
            let value = future.await.map_err(map_promise_err)?;
            let result = serde_wasm_bindgen::from_value(value)?;
            Ok(result)
        })
        .await
    }

    /// Executes a query against the database and returns all rows.
    pub async fn all<T>(&self) -> Result<D1Result<T>>
    where
        T: for<'a> Deserialize<'a>, {
        let future = SendWrapper::new(JsFuture::from(self.0.all()));
        wrap_send(async move {
            let value = future.await.map_err(map_promise_err)?;
            let result = serde_wasm_bindgen::from_value(value)?;
            Ok(result)
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
            let result = future.await.map_err(map_promise_err)?;
            let result = result
                .dyn_into::<Array>()
                .map_err(|_| DatabaseError::JsCast)?;
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

fn wrap_send<Fut, O>(f: Fut) -> Pin<Box<dyn Future<Output = O> + Send + Sync + 'static>>
where
    Fut: Future<Output = O> + 'static, {
    Box::pin(SendWrapper::new(f))
}

fn map_promise_err(err: JsValue) -> DatabaseError {
    let message = err
        .as_string()
        .or_else(|| {
            err.dyn_ref::<js_sys::Error>().map(|e| {
                format!(
                    "{} Message: {} Cause: {:?}",
                    e.to_string(),
                    e.message(),
                    e.cause()
                )
            })
        })
        .unwrap_or_else(|| format!("Unknown Javascript error: {:?}", err));
    DatabaseError::AwaitPromise(message)
}

#[cfg(test)]
mod tests {
    use static_assertions::assert_impl_all;

    use super::*;

    #[allow(dead_code)]
    pub(crate) fn require_send<T: Send>(_t: &T) {}

    #[allow(dead_code)]
    pub(crate) fn require_sync<T: Sync>(_t: &T) {}

    #[allow(dead_code)]
    pub(crate) fn require_unpin<T: Unpin>(_t: &T) {}

    macro_rules! into_todo {
        ($typ:ty) => {{
            let x: $typ = todo!();
            x
        }};
    }
    macro_rules! async_assert_fn_send {
        (Send & $(!)?Sync, $value:expr) => {
            require_send(&$value);
        };
        (!Send & $(!)?Sync, $value:expr) => {
            AmbiguousIfSend::some_item(&$value);
        };
    }
    macro_rules! async_assert_fn_sync {
        ($(!)?Send &Sync, $value:expr) => {
            require_sync(&$value);
        };
        ($(!)?Send & !Sync, $value:expr) => {
            AmbiguousIfSync::some_item(&$value);
        };
    }
    macro_rules! async_assert_fn {
    ($($f:ident $(< $($generic:ty),* > )? )::+($($arg:ty),*): $($tok:tt)*) => {
        #[allow(unreachable_code)]
        #[allow(unused_variables)]
        const _: fn() = || {
            let f = $($f $(::<$($generic),*>)? )::+( $( into_todo!($arg) ),* );
            async_assert_fn_send!($($tok)*, f);
            async_assert_fn_sync!($($tok)*, f);
        };
    };
}

    pub(crate) use {async_assert_fn, async_assert_fn_send, async_assert_fn_sync, into_todo};

    assert_impl_all!(Database: Send, Sync);
    assert_impl_all!(PreparedStatement: Send, Sync);
    assert_impl_all!(D1Result: Send, Sync);

    async_assert_fn!(Database::dump(_): Send & Sync);
    async_assert_fn!(Database::batch<String>(_, _): Send & Sync);
    async_assert_fn!(Database::exec(_, _): Send & Sync);

    async_assert_fn!(PreparedStatement::bind(_, String): Send & Sync);
    async_assert_fn!(PreparedStatement::bind_many(_, _): Send & Sync);
    async_assert_fn!(PreparedStatement::first<String>(_, _): Send & Sync);
    async_assert_fn!(PreparedStatement::run(_): Send & Sync);
    async_assert_fn!(PreparedStatement::all<String>(_): Send & Sync);
    async_assert_fn!(PreparedStatement::raw<String>(_): Send & Sync);
}
