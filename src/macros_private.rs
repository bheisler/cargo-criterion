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
                error!("Error: {:?}", &e);
                let closure = $el;
                return closure();
            }
        }
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

macro_rules! elapsed {
    ($msg:expr, $block:expr) => {{
        let start = ::std::time::Instant::now();
        let out = $block;
        let elapsed = &start.elapsed();

        debug!(
            "{} took {}",
            $msg,
            crate::format::time(crate::DurationExt::to_nanos(elapsed) as f64)
        );

        out
    }};
}
