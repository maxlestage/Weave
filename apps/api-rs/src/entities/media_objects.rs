//! Les octets d'un média, en base.
//!
//! Écrite à la main plutôt que par `sea-orm-codegen`, comme le reste de ce
//! répertoire le sera au prochain passage : la table est née après la
//! génération.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "media_objects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub key: String,
    #[sea_orm(column_name = "accountId", column_type = "Text")]
    pub account_id: String,
    #[sea_orm(column_name = "contentType", column_type = "Text")]
    pub content_type: String,
    #[sea_orm(column_type = "Blob")]
    pub bytes: Vec<u8>,
    #[sea_orm(column_name = "byteSize")]
    pub byte_size: i32,
    #[sea_orm(column_name = "createdAt")]
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::accounts::Entity",
        from = "Column::AccountId",
        to = "super::accounts::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Accounts,
}

impl Related<super::accounts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Accounts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
