pub const fn get_user_agent() -> &'static str {
    concat!("cli-rs", env!("CARGO_PKG_VERSION"))
}
