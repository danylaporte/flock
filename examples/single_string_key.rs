#![feature(proc_macro_hygiene)]

use flock::{failure::Error, locks, relations, Entity};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let locks = locks!(read: [Settings]).await?;

    locks
        .settings
        .iter()
        .take(10)
        .for_each(|setting| println!("{} = {}", setting.key, setting.value));

    Ok(())
}

#[derive(Entity)]
#[table("[dbo].[Settings]")]
pub struct Setting {
    #[key]
    key: String,
    value: String,
}

#[relations]
trait Relations {
    fn relation1(setting: Settings) -> bool {
        setting.iter().any(|_| true)
    }

    fn relation2(setting: Settings) -> bool {
        setting.iter().any(|_| true)
    }
}
