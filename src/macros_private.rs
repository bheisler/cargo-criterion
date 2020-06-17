//! Private macro used for error handling.

/// Logs an error, ignores an `Ok` value.
macro_rules! log_if_err {
    ($x:expr) => {
        let closure = || {
            try_else_return!($x);
        };
        closure();
    };
}

/// Matches a result, returning the `Ok` value in case of success,
/// exits the calling function otherwise.
/// A closure which returns the return value for the function can
/// be passed as second parameter.
macro_rules! try_else_return {
    ($x:expr) => {
        try_else_return!($x, || {});
    };
    ($x:expr, $el:expr) => {
        match $x {
            Ok(x) => x,
            Err(e) => {
                crate::error::log_error(&e);
                let closure = $el;
                return closure();
            }
        }
    };
}

/// Print an error message to stdout. Format is the same as println! or format!
macro_rules! error {
    ($($arg:tt)*) => (
        println!("cargo-criterion ERROR: {}", &format!($($arg)*));
    )
}

macro_rules! warn {
    ($($arg:tt)*) => (
        println!("cargo-criterion WARNING: {}", &format!($($arg)*));
    )
}

/// Print a debug message to stdout. Format is the same as println! or format!
macro_rules! info {
    ($($arg:tt)*) => {
        //if $crate::debug_enabled() {
        //    println!("cargo-criterion DEBUG: {}", &format!($($arg)*));
        //}
    };
}

/// vec! but for pathbufs
macro_rules! path {
    (PUSH $to:expr, $x:expr, $($y:expr),+) => {
        $to.push($x);
        path!(PUSH $to, $($y),+);
    };
    (PUSH $to:expr, $x:expr) => {
        $to.push($x);
    };
    ($x:expr, $($y:expr),+) => {{
        let mut path_buffer = PathBuf::new();
        path!(PUSH path_buffer, $x, $($y),+);
        path_buffer
    }};
}
