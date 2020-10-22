#![feature(proc_macro_hygiene)]

use flock::{locks, Entity, EntityId, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let locks = locks!(read: [UserAccounts]).await?;

    locks
        .user_accounts
        .iter()
        .take(10)
        .for_each(|ua| println!("{}", &ua.user_id));

    Ok(())
}

#[derive(Entity)]
#[table("[dbo].[UserAccounts]")]
pub struct UserAccount {
    #[key]
    user_id: UserId,
    #[key]
    account_id: AccountId,
}

#[derive(EntityId)]
pub struct AccountId(u32);

#[derive(EntityId)]
pub struct UserId(u32);
