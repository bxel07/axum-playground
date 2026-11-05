use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Products::Table)
                    .if_not_exists()
                    .col(pk_auto(Products::Id))
                    .col(string(Products::Name))
                    .col(string(Products::Sku).unique_key())
                    .col(string(Products::Category).default("umum"))
                    .col(decimal_len(Products::Price, 12, 2).default(0))
                    .col(integer(Products::Quantity).default(0))
                    .col(
                        timestamp_with_time_zone(Products::CreatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        timestamp_with_time_zone(Products::UpdatedAt)
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Products::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Products {
    Table,
    Id,
    Name,
    Sku,
    Category,
    Price,
    Quantity,
    CreatedAt,
    UpdatedAt,
}
