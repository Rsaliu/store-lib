use crate::stores::store::{StoreError, StoreTrait};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::PgArguments,
    query::{self, QueryAs},
    Execute, Pool, Postgres,
};
use simple_logger::SimpleLogger;
use std::{error::Error, io};
use user_lib::user::user::{User, UserRoles};
use uuid::Uuid;
use crypto_lib::crypto::{self, crypto::CryptoOp};
use sqlx::query::Query;
use sqlx::postgres::PgRow;
use sqlx::Row;
use sqlx::Column;
use sqlx::TypeInfo;
use std::str::FromStr;
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct UserRow {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub user_role: UserRoles,
    pub confirmed: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
    fn bind_values<'a>(&self,
        custom_query: Query<'a,Postgres, PgArguments>,
        json_value: &'a serde_json::Value,
    ) -> Result<Query<'a,Postgres, PgArguments>, StoreError> {
        // Loop through the JSON object by key
        let mut custom_query = custom_query;

        if let serde_json::Value::Object(map) = json_value {
            for (key, value) in map {
                println!("Key: {}, Value: {}", key, value);
                match key.as_str() {
                    "id" => {
                        let muid:Uuid = serde_json::from_value(value.clone()).map_err(StoreError::JsonError)?;
                        println!("uuid obtained is: {:?}", muid);
                        custom_query = custom_query.bind(muid);
                    },
                    "user_role" => {
                        let role:UserRoles = serde_json::from_value(value.clone()).map_err(StoreError::JsonError)?;
                        println!("role obtained is: {:?}", role);
                        custom_query = custom_query.bind(role);
                    },
                    "created_at"|"updated_at" => {
                        let time:NaiveDateTime = serde_json::from_value(value.clone()).map_err(StoreError::JsonError)?;
                        println!("time obtained is: {:?}", time);
                        custom_query = custom_query.bind(time);
                    },
                    "confirmed" => {
                        let confirm:bool = serde_json::from_value(value.clone()).map_err(StoreError::JsonError)?;
                        println!("confirm obtained is: {:?}", confirm);
                        custom_query = custom_query.bind(confirm);
                    },
                    _ => {
                        // Handle other keys here
                        let text:String = serde_json::from_value(value.clone()).map_err(StoreError::JsonError)?;
                        println!("text obtained is: {:?}", text);
                        custom_query = custom_query.bind(text);
                    }
                }
            }
        }

        Ok(custom_query)
    }
    fn row_to_json(&self,row: &PgRow) -> Result<serde_json::Value, sqlx::Error> {
        let mut json_obj = serde_json::Map::new();
        println!("trying to convert to json");
        for column in row.columns() {
            let column_name = column.name();
            println!("column type is: {:?}",column.type_info().name());
            let column_value: serde_json::Value = match column.type_info().name() {
                // Handle different types as needed
                "UUID" => serde_json::json!(row.try_get::<uuid::Uuid, _>(column_name)?),
                "user_role" => serde_json::json!(row.try_get::<UserRoles, _>(column_name)?),
                "BOOL" => serde_json::json!(row.try_get::<bool, _>(column_name)?),
                "TIMESTAMP" => serde_json::json!(row.try_get::<NaiveDateTime, _>(column_name)?),
                _ => serde_json::json!(row.get::<String, _>(column_name)), 
            };
    
            json_obj.insert(column_name.to_owned(), column_value);
        }
    
        Ok(serde_json::Value::Object(json_obj))
    }
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
    async fn patch(
        &self,
        connection: &Pool<Postgres>,
        id: Uuid,
        patch: serde_json::Value,
    ) -> Result<(), StoreError> {
        let mut custom_query: String = String::from(r#"update users set "#);
        let mut values = Vec::new();
        let mut conditions = Vec::new();
        let mut max_variable = 0;
        // Check if the parsed value is an object
        if let serde_json::Value::Object(map) = patch.clone() {
            // Iterate over the key-value pairs in the object

            for (key, value) in map {
                println!("Key: {}, Value: {}", key, value);
                values.push(value.to_string().trim_matches('"').to_string());
                conditions.push(format!("{} = ${}", key, conditions.len() + 1));
                max_variable = conditions.len() + 1;
            }
        } else {
            log::debug!("The JSON data is not an object");
            return Err(StoreError::NotFound);
        }
        custom_query.push_str(&conditions.join(" , "));
        custom_query.push_str(format!(" WHERE id = ${}", max_variable).as_str());
        println!("final query is: {custom_query}");
        let mut custom_query = sqlx::query(&custom_query);
        custom_query = self.bind_values(custom_query, &patch)?;
        custom_query = custom_query.bind(id);
        println!("id to patch {}", id.to_string());
        let affected_rows = custom_query.execute(connection).await.map_err(StoreError::SqlxError)?;

        println!("Updated {} row(s)", affected_rows.rows_affected());
        Ok(())
    }
}
