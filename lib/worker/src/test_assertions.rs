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
