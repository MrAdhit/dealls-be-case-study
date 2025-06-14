pub use sea_orm_migration::prelude::*;

mod util;
mod m20250613_083042_init;
mod m20250615_063806_generate_users;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250613_083042_init::Migration),
            Box::new(m20250615_063806_generate_users::Migration),
        ]
    }
}
