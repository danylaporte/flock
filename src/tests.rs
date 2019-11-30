use crate::{mssql_client::Connection, ConnOrFactory, LoadFromSql};

type IsKey = bool;
type IsNull = Option<bool>;

/// use in flock_derive::Entity macro to test the loading of a table.
pub async fn test_load<T: LoadFromSql>() {
    let conn = ConnOrFactory::from_env("DB").expect("Environment variable DB");
    T::load_from_sql(conn).await.expect("load fail");
}

/// use in flock_derive::Entity macro to test the schema of a table.
pub async fn test_schema(table: &str, fields: &[(&str, &dyn Fn(&str) -> bool, IsNull, IsKey)]) {
    let conn = Connection::from_env("DB").await.expect("Connection");

    const SQL: &str = r#"SELECT
        c.COLUMN_NAME,
        c.DATA_TYPE,
        CAST(CASE c.IS_NULLABLE WHEN 'YES' THEN 1 ELSE 0 END AS BIT) IS_NULLABLE,
        CAST(ISNULL((
            SELECT TOP 1 1
            FROM
                INFORMATION_SCHEMA.TABLE_CONSTRAINTS tc
                INNER JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE ku
                ON tc.CONSTRAINT_TYPE = 'PRIMARY KEY'
                AND tc.CONSTRAINT_NAME = KU.CONSTRAINT_NAME
            WHERE
                ku.TABLE_SCHEMA = t.TABLE_SCHEMA
                AND ku.TABLE_NAME = t.TABLE_NAME
                AND ku.COLUMN_NAME = c.COLUMN_NAME
        ), 0) AS BIT) IS_PRIMARY_KEY
    FROM
        INFORMATION_SCHEMA.TABLES t
        INNER JOIN INFORMATION_SCHEMA.COLUMNS c
        ON t.TABLE_NAME = c.TABLE_NAME
        AND t.TABLE_SCHEMA = c.TABLE_SCHEMA
    WHERE
        @p1 = '[' + t.TABLE_SCHEMA + '].[' + t.TABLE_NAME + ']'
    "#;

    let mut rows: Vec<(String, String, bool, bool)> =
        conn.query(SQL, table).await.expect("Query").1;

    // make all field name lowercase for comparison
    rows.iter_mut().for_each(|r| r.0 = r.0.to_lowercase());

    if rows.len() == 0 {
        panic!("Table `{}` not found.", table);
    }

    for &(ref f_name, ref f_type, f_null, f_key) in fields {
        let &(_, ref r_type, r_null, r_key) = rows
            .iter()
            .find(|t| f_name == &t.0)
            .unwrap_or_else(|| panic!("`{}` not found.", f_name));

        if r_key && !f_key {
            panic!("`{}` is a key.", f_name);
        }

        if f_key && !r_key {
            panic!("`{}` is not a key.", f_name);
        }

        if !f_type(r_type) {
            panic!("`{}` type invalid.", f_name);
        }

        let f_null = f_null.unwrap_or(r_null);

        if r_null && !f_null {
            panic!("`{}` is nullable.", f_name);
        }

        if f_null && !r_null {
            panic!("`{}` is not nullable.", f_name);
        }
    }

    for &(ref r_name, _, _, r_key) in &rows {
        if r_key && !fields.iter().any(|f| f.0 == r_name) {
            panic!("missing `{}` key field.", r_name);
        }
    }
}
