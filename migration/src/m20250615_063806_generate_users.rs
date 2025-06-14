use sea_orm_migration::prelude::*;
use sha2::Digest as _;

use crate::m20250613_083042_init::User;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let time = Expr::val("2025-06-15T06:58:41.474Z").cast_as("timestamptz");

        // Creates 100 employees
        for i in 1..=100 {
            let uuid = format!("{:032x}", i as u128);
            let username = i.to_string();
            let salary = rand::random_range(5_000_000..=20_000_000);

            let hashed_password = &sha2::Sha256::digest(&format!("{}:{}", username, username))[..];

            manager
                .exec_stmt(Query::insert()
                    .into_table(User::Table)
                    .columns(["id", "created_at", "updated_at", "username", "password", "role", "salary"])
                    .values_panic([Expr::val(uuid).cast_as("uuid"), time.clone(), time.clone(), username.into(), hashed_password.into(), Expr::val("employee").cast_as("role_type"), salary.into()])
                    .to_owned()
            ).await.unwrap();
        }
        
        // Create an admin

        let hashed_password = &sha2::Sha256::digest("admin:admin")[..];

        manager
            .exec_stmt(Query::insert()
                .into_table(User::Table)
                .columns(["id", "created_at", "updated_at", "username", "password", "role", "salary"])
                .values_panic([Expr::val(format!("{:032x}", 12345 as u128)).cast_as("uuid"), time.clone(), time.clone(), "admin".into(), hashed_password.into(), Expr::val("admin").cast_as("role_type"), 0.into()])
                .to_owned()
        ).await.unwrap();
            
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for i in 1..=100 {
            let uuid = format!("{:032x}", i as u128);

            manager
                .exec_stmt(Query::delete()
                    .from_table(User::Table)
                    .and_where(Expr::col("id").eq(Expr::val(uuid).cast_as("uuid")))
                    .to_owned()
            ).await.unwrap();
        }

        manager
            .exec_stmt(Query::delete()
                .from_table(User::Table)
                .and_where(Expr::col("id").eq(Expr::val(format!("{:032x}", 12345 as u128)).cast_as("uuid")))
                .to_owned()
        ).await.unwrap();

        Ok(())
    }
}
