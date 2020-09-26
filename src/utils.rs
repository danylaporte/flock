use mssql_client::{FromColumn, Result, Row};

pub fn read_5_fields<'a, A, B, C, D, E>(
    row: &'a Row,
    mut index: usize,
    names: [&'static str; 5],
) -> Result<(A, B, C, D, E)>
where
    A: FromColumn<'a>,
    B: FromColumn<'a>,
    C: FromColumn<'a>,
    D: FromColumn<'a>,
    E: FromColumn<'a>,
{
    let a = row.get_named_err(index, names[index])?;

    index += 1;
    let b = row.get_named_err(index, names[index])?;

    index += 1;
    let c = row.get_named_err(index, names[index])?;

    index += 1;
    let d = row.get_named_err(index, names[index])?;

    index += 1;
    let e = row.get_named_err(index, names[index])?;

    Ok((a, b, c, d, e))
}
