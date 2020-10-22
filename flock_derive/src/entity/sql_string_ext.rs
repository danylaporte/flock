pub(super) trait SqlStringExt {
    fn add(&mut self, s: &str) -> &mut Self;
    fn add_sep(&mut self, sep: &str) -> &mut Self;
    fn add_diff(&mut self, field: &str, index: usize) -> &mut Self;
    fn add_field(&mut self, field: &str) -> &mut Self;
    fn add_field_with_alias(&mut self, field: &str, table_alias: &str) -> &mut Self;
    fn add_param(&mut self, index: usize) -> &mut Self;
}

impl SqlStringExt for String {
    fn add(&mut self, s: &str) -> &mut Self {
        self.push_str(s);
        self
    }

    fn add_sep(&mut self, sep: &str) -> &mut Self {
        if !self.is_empty() {
            self.push_str(sep)
        }
        self
    }

    fn add_diff(&mut self, field: &str, index: usize) -> &mut Self {
        let s = format!(
            "[{field}] != @p{index} OR ([{field}] IS NULL AND @p{index} IS NOT NULL) OR ([{field}] IS NOT NULL AND @p{index} IS NULL)",
            field = field,
            index = index,
        );
        self.push_str(&s);
        self
    }

    fn add_field(&mut self, field: &str) -> &mut Self {
        self.push('[');
        self.push_str(field);
        self.push(']');
        self
    }

    fn add_field_with_alias(&mut self, field: &str, table_alias: &str) -> &mut Self {
        self.add(table_alias).add(".").add_field(field)
    }

    fn add_param(&mut self, index: usize) -> &mut Self {
        self.push_str("@p");
        self.push_str(&index.to_string());
        self
    }
}
