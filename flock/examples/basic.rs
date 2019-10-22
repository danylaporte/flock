#![feature(proc_macro_hygiene)]

use flock_derive::{locks, Entity, EntityId};
use futures::Future;
use tokio::executor::current_thread::block_on_all;

fn main() {
    let fut = locks!(read: [Accounts]).map(|locks| {
        locks
            .accounts
            .iter()
            .take(10)
            .for_each(|account| println!("{}", &account.name));
    });

    block_on_all(fut).unwrap();
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
