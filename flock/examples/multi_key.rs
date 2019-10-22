#![feature(proc_macro_hygiene)]

use flock_derive::{locks, Entity, EntityId};
use futures::Future;
use tokio::executor::current_thread::block_on_all;

fn main() {
    let fut = locks!(read: [UserAccounts]).map(|locks| {
        locks
            .user_accounts
            .iter()
            .take(10)
            .for_each(|ua| println!("{}", &ua.user_id));
    });

    block_on_all(fut).unwrap();
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
