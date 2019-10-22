macro_rules! try_token_stream {
    ($v:expr) => {
        match $v {
            Ok(v) => v,
            Err(e) => return e,
        }
    };
}
