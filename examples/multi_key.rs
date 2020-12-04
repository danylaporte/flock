use flock::{locks_await, Entity, EntityId, Result};

#[tokio::main]
async fn main() -> Result<()> {
    locks_await!(read: [UserAccounts]);

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
