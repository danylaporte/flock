#![feature(proc_macro_hygiene)]

use flock::{locks, Entity, EntityId, MergeSql, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let locks = locks!(read: [Accounts]).await?;

    locks
        .accounts
        .iter()
        .take(10)
        .for_each(|account| println!("{}", &account.name));

    Ok(())
}

#[derive(EntityId)]
struct AccountId(u32);

#[derive(Entity)]
#[table("[dbo].[Accounts]")]
#[where_clause("[NAME] IS NOT NULL")]
struct Account {
    #[key]
    id: AccountId,
    name: String,
}
