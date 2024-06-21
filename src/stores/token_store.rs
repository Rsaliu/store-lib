use std::str::FromStr;

use crate::stores::store::{StoreError, StoreTrait};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, Execute, Pool, Postgres};
use user_lib::user::user::{User, UserRoles};
use uuid::Uuid;
use token_lib::token::token::Token;
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct TokenRow {
    id: Uuid,
    token_string: String,
    user_id: Uuid,
    expired_in: NaiveDateTime,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl Into<Token> for TokenRow {
    fn into(self) -> Token {
        Token::new_full(
            self.id,
            self.token_string,
            self.user_id,
            self.expired_in
        )
    }
}

#[derive(Debug, Default)]
pub struct TokenPGStore;

impl TokenPGStore{
    pub async fn get_by_userid(
        &self,
        connection: &Pool<Postgres>,
        user_id: Uuid,
    ) -> Result<Vec<serde_json::Value>, StoreError> {
        let rows = sqlx::query_as!(TokenRow, r#"SELECT * FROM tokens WHERE user_id = $1"#, user_id)
                .fetch_optional(connection)
                .await
                .map_err(StoreError::SqlxError)?;
        
        let token_datas = rows
        .iter()
        .map(|row| serde_json::to_value(row.clone()).map_err(StoreError::JsonError))
        .collect::<Result<Vec<serde_json::Value>, StoreError>>()?;
        Ok(token_datas)
    }
}


impl StoreTrait for TokenPGStore {
    async fn insert(
        &self,
        connection: &Pool<Postgres>,
        item: serde_json::Value,
    ) -> Result<(), StoreError> {
        let token_obj: Token = serde_json::from_value(item).map_err(StoreError::JsonError)?;

        let token = token_obj.get_token().to_string();
        let user_id = token_obj.get_user_id();
        let expiry = token_obj.get_expired_time();
        // TODO
        // Store Hash instead of clear password, will be done after implementation of crypto library
        sqlx::query!(
            // language=PostgreSQL
            r#"
                    insert into "tokens"(token_string,user_id,expired_in)
                    values ($1, $2, $3)"#,
            token,
            user_id,
            expiry
        )
        .execute(connection)
        .await
        .map_err(StoreError::SqlxError)?;
        Ok(())
    }

    async fn get(
        &self,
        connection: &Pool<Postgres>,
        id: Uuid,
    ) -> Result<Vec<serde_json::Value>, StoreError> {
        let rows = sqlx::query_as!(TokenRow, r#"SELECT * FROM tokens WHERE id = $1"#, id)
            .fetch_all(connection)
            .await
            .map_err(StoreError::SqlxError)?;
        let token_datas = rows
            .iter()
            .map(|row| serde_json::to_value(row.clone()).map_err(StoreError::JsonError))
            .collect::<Result<Vec<serde_json::Value>, StoreError>>()?;
        Ok(token_datas)
    }

    async fn delete(&self, connection: &Pool<Postgres>, id: Uuid) -> Result<(), StoreError> {
        sqlx::query!(
            // language=PostgreSQL
            r#"
                    delete from  tokens where id=$1"#,
            id
        )
        .execute(connection)
        .await
        .map_err(StoreError::SqlxError)?;
        Ok(())
    }

    async fn update(
        &self,
        connection: &Pool<Postgres>,
        id: Uuid,
        item: serde_json::Value,
    ) -> Result<(), StoreError> {
        let token_data: Token = serde_json::from_value(item).map_err(StoreError::JsonError)?;
        let naive_now: NaiveDateTime = Utc::now().naive_utc();
        sqlx::query!(
            // language=PostgreSQL
            r#"
                update tokens set token_string = $1, user_id = $2, expired_in = $3, updated_at = $4 where id=$5"#,
            token_data.get_token(),
            token_data.get_user_id(),
            token_data.get_expired_time(),
            naive_now,
            id
        )
        .execute(connection)
        .await
        .map_err(StoreError::SqlxError)?;
        Ok(())
    }

    async fn get_by_slug(
        &self,
        connection: &Pool<Postgres>,
        json_slug: serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, StoreError> {
        let mut custom_query: String = String::from(r#"SELECT * FROM tokens WHERE "#);
        let mut values = Vec::new();
        let mut keys = Vec::new();
        let mut conditions = Vec::new();
        // Check if the parsed value is an object
        if let serde_json::Value::Object(map) = json_slug {
            // Iterate over the key-value pairs in the object

            for (key, value) in map {
                println!("Key: {}, Value: {}", key, value);
                values.push(value.to_string().trim_matches('"').to_string());
                keys.push(key.clone());
                conditions.push(format!("{} = ${}", key, conditions.len() + 1));
            }
        } else {
            log::debug!("The JSON data is not an object");
            return Err(StoreError::NotFound);
        }
        custom_query.push_str(&conditions.join(" AND "));
        println!("final query is: {custom_query}");
        let mut custom_query = sqlx::query_as::<_, TokenRow>(&custom_query);
        let mut index:usize = 0;
        for value in values {
            match keys[index].as_str() {
                "user_id" => {
                    let muid = Uuid::from_str(&value).map_err(StoreError::UUIDError)?;
                    println!("uuid obtained is: {:?}",muid);
                    custom_query = custom_query.bind(muid);
                },
                _ =>{
                    custom_query = custom_query.bind(value.clone());
                }
            }
            println!("value to bind is: {value}");
            index+=1;
        }
        //print!("final query is: {:?}",custom_query);
        let rows = custom_query
            .fetch_all(connection)
            .await
            .map_err(StoreError::SqlxError)?;
        let user_datas = rows
            .iter()
            .map(|row| serde_json::to_value(row.clone()).map_err(StoreError::JsonError))
            .collect::<Result<Vec<serde_json::Value>, StoreError>>()?;
        Ok(user_datas)
    }

    async fn count(&self, connection: &Pool<Postgres>) -> Result<usize, StoreError> {
        let count: Option<i64> = sqlx::query_scalar(
            // language=PostgreSQL
            r#"
                    SELECT COUNT(id) FROM tokens"#,
        )
        .fetch_one(connection)
        .await
        .map_err(StoreError::SqlxError)?;

        let count = if let Some(count) = count { count } else { 0 };
        Ok(count as usize)
    }
    async fn get_all_paginate(
        &self,
        connection: &Pool<Postgres>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<serde_json::Value>, StoreError> {
        let rows = sqlx::query_as!(
            TokenRow,
             r#"SELECT * FROM tokens order by id asc limit $1 offset $2"#,
             limit,offset)
            .fetch_all(connection)
            .await
            .map_err(StoreError::SqlxError)?;
        let user_datas = rows
            .iter()
            .map(|row| serde_json::to_value(row.clone()).map_err(StoreError::JsonError))
            .collect::<Result<Vec<serde_json::Value>, StoreError>>()?;
        Ok(user_datas)
    }
}


