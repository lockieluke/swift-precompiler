pub struct Expression;

impl Expression {
    pub const INCLUDE_STR_RGX: &'static str =
        r#"precompileIncludeStr\s*\(\s*["']([^"']+)["']\s*\)"#;

    pub const INCLUDE_DATA_RGX: &'static str =
        r#"precompileIncludeData\s*\(\s*["']([^"']+)["']\s*\)"#;
}
