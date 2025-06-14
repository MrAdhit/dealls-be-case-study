use sea_orm_migration::{prelude::{extension::postgres::TypeDropStatement, *}, sea_orm::{ActiveEnum, DbBackend, DeriveActiveEnum, EnumIter, Schema}};

use crate::{setup_user_table_fk, util::{default_table_statement, default_user_table_statement, DefaultColumn}};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let schema = Schema::new(DbBackend::Postgres);

        manager
            .create_type(
                schema.create_enum_from_active_enum::<RoleType>()
            ).await.unwrap();

        manager
            .create_table(default_table_statement()
                .table(User::Table)
                .col(ColumnDef::new(User::Username)
                    .text()
                    .unique_key()
                    .not_null())
                .col(ColumnDef::new(User::Password)
                    .binary()
                    .not_null()) // Password should be in a hashed format
                .col(ColumnDef::new(User::Salary)
                    .big_integer()
                    .not_null()) // In a perfect world we would use u64 / unsigned big int, but PostgreSQL doesn't support unsigned integer...
                .col(ColumnDef::new(User::Role)
                    .custom(RoleType::name())
                    .not_null())
                .take()
            ).await.unwrap();
        
        manager
            .create_table(default_user_table_statement()
                .table(AttendancePeriod::Table)
                .col(ColumnDef::new(AttendancePeriod::StartAt)
                    .timestamp_with_time_zone()
                    .not_null())
                .col(ColumnDef::new(AttendancePeriod::EndAt)
                    .timestamp_with_time_zone()
                    .not_null())
                .col(ColumnDef::new(AttendancePeriod::Processed)
                    .boolean()
                    .not_null()
                    .default(false))
                .take()
            ).await.unwrap();
        setup_user_table_fk!(manager, AttendancePeriod::Table);
        
        manager
            .create_table(default_user_table_statement()
                .table(EmployeeAttendance::Table)
                .col(ColumnDef::new(EmployeeAttendance::AttendancePeriodId)
                    .uuid()
                    .not_null())
                .take()
        ).await.unwrap();
        setup_user_table_fk!(manager, EmployeeAttendance::Table);

        manager.create_foreign_key(ForeignKeyCreateStatement::new()
            .from(EmployeeAttendance::Table, EmployeeAttendance::AttendancePeriodId)
            .to(AttendancePeriod::Table, DefaultColumn::Id)
            .take()
        ).await.unwrap();
        
        manager
            .create_table(default_user_table_statement()
                .table(EmployeeOvertime::Table)
                .col(ColumnDef::new(EmployeeOvertime::ExtraHours)
                    .tiny_integer()
                    .not_null())
                .col(ColumnDef::new(EmployeeOvertime::AttendancePeriodId)
                    .uuid()
                    .not_null())
                .take()
        ).await.unwrap();
        setup_user_table_fk!(manager, EmployeeOvertime::Table);

        manager.create_foreign_key(ForeignKeyCreateStatement::new()
            .from(EmployeeOvertime::Table, EmployeeOvertime::AttendancePeriodId)
            .to(AttendancePeriod::Table, DefaultColumn::Id)
            .take()
        ).await.unwrap();
        
        manager
            .create_table(default_user_table_statement()
                .table(EmployeeReimbursement::Table)
                .col(ColumnDef::new(EmployeeReimbursement::Amount)
                    .big_integer()
                    .not_null())
                .col(ColumnDef::new(EmployeeReimbursement::Description)
                    .text()
                    .not_null())
                .col(ColumnDef::new(EmployeeReimbursement::AttendancePeriodId)
                    .uuid()
                    .not_null())
                .take()
        ).await.unwrap();
        setup_user_table_fk!(manager, EmployeeReimbursement::Table);
        
        manager.create_foreign_key(ForeignKeyCreateStatement::new()
            .from(EmployeeReimbursement::Table, EmployeeReimbursement::AttendancePeriodId)
            .to(AttendancePeriod::Table, DefaultColumn::Id)
            .take()
        ).await.unwrap();

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(
            TableDropStatement::new()
                .table(EmployeeAttendance::Table)
                .take()
        ).await.unwrap();

        manager.drop_table(
            TableDropStatement::new()
                .table(EmployeeOvertime::Table)
                .take()
        ).await.unwrap();

        manager.drop_table(
            TableDropStatement::new()
                .table(EmployeeReimbursement::Table)
                .take()
        ).await.unwrap();

        manager.drop_table(
            TableDropStatement::new()
                .table(AttendancePeriod::Table)
                .take()
        ).await.unwrap();


        manager
            .drop_table(
                TableDropStatement::new()
                    .table(User::Table)
                    .take()
            ).await.unwrap();
        
        manager
            .drop_type(
                TypeDropStatement::new()
                    .name(RoleType::name())
                    .to_owned()
            ).await.unwrap();
        
        Ok(())
    }
}

#[derive(Iden)]
pub(crate) enum User {
    Table,
    Username,
    Password,
    Role,
    Salary,
}

#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "role_type")]
enum RoleType {
    #[sea_orm(string_value = "employee")]
    Employee,
    #[sea_orm(string_value = "admin")]
    Admin,
}

#[derive(Iden)]
enum AttendancePeriod {
    Table,
    StartAt,
    EndAt,
    Processed,
}

#[derive(Iden)]
enum EmployeeAttendance {
    Table,
    AttendancePeriodId,
}

#[derive(Iden)]
enum EmployeeOvertime {
    Table,
    ExtraHours,
    AttendancePeriodId,
}

#[derive(Iden)]
enum EmployeeReimbursement {
    Table,
    Amount,
    Description,
    AttendancePeriodId,
}
