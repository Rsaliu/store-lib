
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::{io,error::Error};
use crate::stores::user_store::UserPGStore;
use crate::stores::token_store::TokenPGStore;
use uuid::Uuid;

#[derive(Debug)]
pub enum StoreError{
    SqlxError(sqlx::Error),
    JsonError(serde_json::Error),
    UUIDError(uuid::Error),
    NotFound,
    OtherError(Box<dyn std::error::Error>)
}

pub enum Store{
    UserPostgresStore(UserPGStore),
    TokenPostgersStore(TokenPGStore)
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StoreError::SqlxError(e) => write!(f, "SqlxError: {}", e),
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


pub trait StoreTrait{
    async fn insert(&self,connection:&Pool<Postgres> ,item:serde_json::Value)->Result<(),StoreError>;
    async fn get(&self,connection:&Pool<Postgres> ,id:Uuid)->Result<Vec<serde_json::Value>,StoreError>;
    async fn get_all_paginate(&self,connection:&Pool<Postgres>,limit:i64,offset:i64)->Result<Vec<serde_json::Value>,StoreError>;
    async fn count(&self,connection:&Pool<Postgres>)->Result<usize,StoreError>;
    //async fn get_many_by_slug(&self,connection:&Pool<Postgres> ,json_slug:serde_json::Value)->Result<Vec<serde_json::Value>,StoreError>;
    async fn get_by_slug(&self,connection:&Pool<Postgres> , json_slug:serde_json::Value)->Result<Vec<serde_json::Value>,StoreError>;
    async fn delete(&self,connection:&Pool<Postgres> ,id:Uuid)->Result<(),StoreError>;
    async fn update(&self,connection:&Pool<Postgres> ,id:Uuid,item:serde_json::Value)->Result<(),StoreError>;
}