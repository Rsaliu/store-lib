use crate::stores::store::{StoreError, StoreTrait};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;
use sqlx::{query, Execute, Pool, Postgres};
use std::{error::Error, io};
use user_lib::user::user::{User, UserRoles};
use uuid::Uuid;
use crypto_lib::crypto::{self, crypto::CryptoOp};

#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct UserRow {
    id: Uuid,
    username: String,
    email: String,
    password_hash: String,
    user_role: UserRoles,
    confirmed: bool,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

#[derive(Debug, Default)]
pub struct UserPGStore;

impl Into<User> for UserRow {
    fn into(self) -> User {
        User::new_full(
            self.id,
            self.username,
            self.email,
            self.password_hash,
            self.user_role,
            self.confirmed,
        )
    }
}

impl UserPGStore {
    pub async fn get_by_username(
        &self,
        connection: &Pool<Postgres>,
        username: &str,
    ) -> Result<serde_json::Value, StoreError> {
        let row = sqlx::query_as!(UserRow, r#"SELECT id,username,email,password_hash, user_role AS "user_role!: UserRoles",confirmed,created_at,updated_at FROM users WHERE username = $1"#, username)
                .fetch_optional(connection)
                .await
                .map_err(StoreError::SqlxError)?
                .ok_or_else(|| StoreError::NotFound)?;
        let user_data = serde_json::to_value(&row).expect("row conversion failed");
        Ok(user_data)
    }
}

impl StoreTrait for UserPGStore {
    async fn insert(
        &self,
        connection: &Pool<Postgres>,
        item: serde_json::Value,
    ) -> Result<(), StoreError> {
        let user_obj: User = serde_json::from_value(item).map_err(StoreError::JsonError)?;

        let name = user_obj.get_name().to_string();
        let password = user_obj.get_password().to_string();
        let crypto_op = CryptoOp::default();
        let password = crypto_op.generate_hash(password).await.map_err(StoreError::OtherError)?;
        let user_role = user_obj.get_role();
        let email = user_obj.get_email();
        let confirmed = user_obj.get_confirmed_status();
        // TODO
        // Store Hash instead of clear password, will be done after implementation of crypto library
        sqlx::query!(
            // language=PostgreSQL
            r#"
                    insert into "users"(username,email, password_hash,user_role,confirmed)
                    values ($1, $2, $3,$4,$5)"#,
            name,
            email,
            password,
            user_role as UserRoles,
            confirmed
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
        let rows = sqlx::query_as!(UserRow, r#"SELECT id,username,email,password_hash, user_role AS "user_role!: UserRoles",confirmed,created_at,updated_at FROM users WHERE id = $1"#, id)
            .fetch_all(connection)
            .await
            .map_err(StoreError::SqlxError)?;
        let user_datas = rows
            .iter()
            .map(|row| serde_json::to_value(row.clone()).map_err(StoreError::JsonError))
            .collect::<Result<Vec<serde_json::Value>, StoreError>>()?;
        Ok(user_datas)
    }

    async fn delete(&self, connection: &Pool<Postgres>, id: Uuid) -> Result<(), StoreError> {
        sqlx::query!(
            // language=PostgreSQL
            r#"
                    delete from  users where id=$1"#,
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
        let user_data: User = serde_json::from_value(item).map_err(StoreError::JsonError)?;
        let naive_now: NaiveDateTime = Utc::now().naive_utc();
        sqlx::query!(
            // language=PostgreSQL
            r#"
                update users set username = $1, email = $2, password_hash =$3, user_role = $4, confirmed = $5, updated_at = $6 where id=$7"#,
            user_data.get_name(),
            user_data.get_email(),
            user_data.get_password(),
            user_data.get_role() as UserRoles,
            user_data.get_confirmed_status(),
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
        let mut custom_query: String = String::from(r#"SELECT * FROM users WHERE "#);
        let mut values = Vec::new();
        let mut conditions = Vec::new();
        // Check if the parsed value is an object
        if let serde_json::Value::Object(map) = json_slug {
            // Iterate over the key-value pairs in the object

            for (key, value) in map {
                println!("Key: {}, Value: {}", key, value);
                values.push(value.to_string().trim_matches('"').to_string());
                conditions.push(format!("{} = ${}", key, conditions.len() + 1));
            }
        } else {
            log::debug!("The JSON data is not an object");
            return Err(StoreError::NotFound);
        }
        custom_query.push_str(&conditions.join(" AND "));
        println!("final query is: {custom_query}");
        let mut custom_query = sqlx::query_as::<_, UserRow>(&custom_query);
        for value in values {
            custom_query = custom_query.bind(value.clone());
            println!("value to bind is: {value}");
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
                    SELECT COUNT(id) FROM users"#,
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
            UserRow,
             r#"SELECT id,username,email,password_hash, user_role AS "user_role!: UserRoles",confirmed,created_at,updated_at FROM users order by id asc limit $1 offset $2"#,
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
