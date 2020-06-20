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
