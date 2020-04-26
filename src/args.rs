use std::ffi::OsString;

pub struct CargoArguments {}
impl CargoArguments {
    pub fn to_arguments(&self) -> Vec<OsString> {
        vec![]
    }
}
