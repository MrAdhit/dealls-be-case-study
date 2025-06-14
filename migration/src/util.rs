use sea_orm_migration::prelude::*;

pub(crate) fn default_table_statement() -> TableCreateStatement {
    TableCreateStatement::new()
        .if_not_exists()
        .col(ColumnDef::new(DefaultColumn::Id)
            .uuid()
            .primary_key()
            .default(Expr::cust("GEN_RANDOM_UUID()"))
            .take())
        .col(ColumnDef::new(DefaultColumn::CreatedAt)
            .timestamp_with_time_zone()
            .not_null()
            .take())
        .col(ColumnDef::new(DefaultColumn::UpdatedAt)
            .timestamp_with_time_zone()
            .not_null()
            .take())
        .take()
}

#[derive(DeriveIden)]
pub(crate) enum DefaultColumn {
    Id,
    CreatedAt,
    UpdatedAt,
}

/// Must run `setup_user_table_fk` macro on the table afterwards
///
/// # Example
///
/// ```rs
/// manager
///     .create_table(default_user_table_statement()
///         .table(AttendancePeriod::Table)
///         .col(ColumnDef::new(AttendancePeriod::StartAt)
///             .date_time()
///             .not_null())
///         .col(ColumnDef::new(AttendancePeriod::EndAt)
///             .date_time()
///             .not_null())
///         .col(ColumnDef::new(AttendancePeriod::Processed)
///             .boolean()
///             .default(false))
///         .take()
///     ).await.unwrap();
/// setup_user_table_fk!(manager, AttendancePeriod::Table);
/// ```
pub(crate) fn default_user_table_statement() -> TableCreateStatement {
    default_table_statement()
        .col(ColumnDef::new(DefaultUserColumn::CreatedBy)
            .uuid())
        .col(ColumnDef::new(DefaultUserColumn::UpdatedBy)
            .uuid())
        .take()
}

#[macro_export]
macro_rules! setup_user_table_fk {
    ($m:expr,$t:expr) => {{
        use crate::util::*;
        use crate::m20250613_083042_init::User;

        $m.create_foreign_key(ForeignKeyCreateStatement::new()
                .from($t, DefaultUserColumn::CreatedBy)
                .to(User::Table, DefaultColumn::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .on_update(ForeignKeyAction::Cascade)
                .take()
        ).await.unwrap();

        $m.create_foreign_key(ForeignKeyCreateStatement::new()
                .from($t, DefaultUserColumn::UpdatedBy)
                .to(User::Table, DefaultColumn::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .on_update(ForeignKeyAction::Cascade)
                .take()
        ).await.unwrap();
    }};
}

#[derive(DeriveIden)]
pub(crate) enum DefaultUserColumn {
    CreatedBy,
    UpdatedBy,
}
