use chrono::offset::TimeZone;
use chrono::Datelike;
use send_wrapper::SendWrapper;
use wasm_bindgen::JsValue;

/// The equivalent to a JavaScript `Date` Object.
/// ```ignore
/// let now = Date::now();
/// let millis = now.as_millis();
/// // or use a specific point in time:
/// let t1: Date = DateInit::Millis(1630611511000).into();
/// let t2: Date = DateInit::String("Thu, 02 Sep 2021 19:38:31 GMT".to_string()).into();
/// ```
#[derive(Debug, Clone)]
pub struct Date(SendWrapper<js_sys::Date>);

impl PartialEq for Date {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_f64() == other.0.as_f64()
    }
}

impl Eq for Date {}

/// Initialize a `Date` by constructing this enum.
/// ```ignore
/// let t1: Date = DateInit::Millis(1630611511000).into();
/// let t2: Date = DateInit::String("Thu, 02 Sep 2021 19:38:31 GMT".to_string()).into();
/// ```
#[derive(Debug)]
pub enum DateInit {
    Millis(u64),
    String(String),
}

impl From<DateInit> for Date {
    fn from(init: DateInit) -> Self {
        Date::new(init)
    }
}

impl Date {
    /// Create a new Date, which requires being initialized from a known
    /// DateInit value.
    pub fn new(init: DateInit) -> Self {
        let val = match init {
            DateInit::Millis(n) => JsValue::from_f64(n as f64),
            DateInit::String(s) => JsValue::from_str(&s),
        };

        Self(SendWrapper::new(js_sys::Date::new(&val)))
    }

    /// Get the current time, represented by a Date.
    pub fn now() -> Self {
        Self(SendWrapper::new(js_sys::Date::new_0()))
    }

    /// Convert a Date into its number of milliseconds since the Unix epoch.
    pub fn as_millis(&self) -> u64 {
        self.0.get_time() as u64
    }
}

impl ToString for Date {
    fn to_string(&self) -> String {
        self.0.to_string().into()
    }
}

#[allow(deprecated)]
impl<T: TimeZone> From<chrono::Date<T>> for Date {
    fn from(d: chrono::Date<T>) -> Self {
        Self(SendWrapper::new(js_sys::Date::new_with_year_month_day(
            d.year() as u32,
            d.month() as i32 - 1,
            d.day() as i32,
        )))
    }
}

impl<T: TimeZone> From<chrono::DateTime<T>> for Date {
    fn from(dt: chrono::DateTime<T>) -> Self {
        DateInit::Millis(dt.timestamp_millis() as u64).into()
    }
}

impl From<Date> for js_sys::Date {
    fn from(val: Date) -> Self {
        val.0.take()
    }
}

impl From<js_sys::Date> for Date {
    fn from(js_date: js_sys::Date) -> Self {
        Self(SendWrapper::new(js_date))
    }
}
