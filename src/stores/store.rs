use crate::stores::token_store::TokenPGStore;
use crate::stores::user_store::UserPGStore;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::Row;
use sqlx::{
    postgres::PgArguments,
    query::{self, Query, QueryAs},
    Execute, Pool, Postgres,
};
use std::{error::Error, io};
use uuid::Uuid;

#[derive(Debug)]
pub enum StoreError {
    SqlxError(sqlx::Error),
    JsonError(serde_json::Error),
    UUIDError(uuid::Error),
    NotFound,
    OtherError(Box<dyn std::error::Error>),
}

pub enum Store {
    UserPostgresStore(UserPGStore),
    TokenPostgersStore(TokenPGStore),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StoreError::SqlxError(e) => match e {
                sqlx::Error::Database(dbe) if dbe.constraint() == Some("users_username_key") => {
                    write!(f, "Store Error: username taken",)
                }
                sqlx::Error::Database(dbe) if dbe.constraint() == Some("users_email_key") => {
                    write!(f, "Store Error: email taken",)
                }
                _ => {
                    write!(f, "Store Error: {}", e)
                }
            },
            StoreError::NotFound => write!(f, "NotFound"),
            StoreError::JsonError(e) => write!(f, "JSon Error: {}", e),
            StoreError::UUIDError(e) => write!(f, "UUID Error: {}", e),
            StoreError::OtherError(e) => write!(f, "Other Error: {}", e),
        }
    }
}

impl std::error::Error for StoreError {}

// impl From<io::Error> for StoreError {
//     fn from(error: io::Error) -> Self {
//         StoreError {
//             kind: String::from("io"),
//             message: error.to_string(),
//         }
//     }
// }

pub trait StoreTrait {
    fn insert(
        &self,
        connection: &Pool<Postgres>,
        item: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<(), StoreError>> + Send;
    fn get(
        &self,
        connection: &Pool<Postgres>,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, StoreError>> + Send;
    fn get_all_paginate(
        &self,
        connection: &Pool<Postgres>,
        limit: i64,
        offset: i64,
    ) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, StoreError>> + Send;
    fn count(
        &self,
        connection: &Pool<Postgres>,
    ) -> impl std::future::Future<Output = Result<usize, StoreError>> + Send;
    //async fn get_many_by_slug(&self,connection:&Pool<Postgres> ,json_slug:serde_json::Value)->Result<Vec<serde_json::Value>,StoreError>;
    fn get_by_slug(
        &self,
        connection: &Pool<Postgres>,
        json_slug: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, StoreError>> + Send;
    fn delete(
        &self,
        connection: &Pool<Postgres>,
        id: Uuid,
    ) -> impl std::future::Future<Output = Result<(), StoreError>> + Send;
    fn update(
        &self,
        connection: &Pool<Postgres>,
        id: Uuid,
        item: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<(), StoreError>> + Send;
    fn patch(
        &self,
        connection: &Pool<Postgres>,
        id: Uuid,
        patch: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<(), StoreError>> + Send;
    fn bind_values<'a>(
        &self,
        custom_query: Query<'a, Postgres, PgArguments>,
        json_value: &'a serde_json::Value,
    ) -> Result<Query<'a, Postgres, PgArguments>, StoreError>;
    fn row_to_json(&self, row: &PgRow) -> Result<serde_json::Value, sqlx::Error>;
}
