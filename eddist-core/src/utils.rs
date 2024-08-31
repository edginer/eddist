pub fn is_prod() -> bool {
    matches!(
        std::env::var("RUST_ENV").as_deref(),
        Ok("prod" | "production")
    )
}
