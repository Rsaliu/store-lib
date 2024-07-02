use std::str::FromStr;

use crate::stores::store::{StoreError, StoreTrait};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::PgArguments,
    query::{self, QueryAs},
    Execute, Pool, Postgres,
};
use token_lib::token::token::{Token, TokenType};
use user_lib::user::user::{User, UserRoles};
use uuid::Uuid;
use sqlx::query::Query;
use sqlx::postgres::PgRow;
use sqlx::Row;
use sqlx::Column;
use sqlx::TypeInfo;
#[derive(Debug, Deserialize, sqlx::FromRow, Serialize, Clone)]
pub struct TokenRow {
    pub id: Uuid,
    pub token_string: String,
    pub token_type: TokenType,
    pub blacklisted: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Into<Token> for TokenRow {
    fn into(self) -> Token {
        Token::new_full(
            self.id,
            self.token_string,
            self.token_type,
            self.blacklisted,
        )
    }
}

#[derive(Debug, Default)]
pub struct TokenPGStore;
impl TokenPGStore{
    pub async fn delete_by_token(&self, connection: &Pool<Postgres>, token_string: String) -> Result<(), StoreError> {
        sqlx::query!(
            // language=PostgreSQL
            r#"
                    delete from  tokens where token_string=$1"#,
            token_string
        )
        .execute(connection)
        .await
        .map_err(StoreError::SqlxError)?;
        Ok(())
    }
}

impl StoreTrait for TokenPGStore {
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
                    "token_type" =>{
                        let token_type:TokenType = serde_json::from_value(value.clone()).map_err(StoreError::JsonError)?;
                        println!("token_type obtained is: {:?}", token_type);
                        custom_query = custom_query.bind(token_type);
                    }
                    "created_at"|"updated_at"=> {
                        let time:NaiveDateTime = serde_json::from_value(value.clone()).map_err(StoreError::JsonError)?;
                        println!("time obtained is: {:?}", time);
                        custom_query = custom_query.bind(time);
                    },
                    "blacklisted" => {
                        let blacklisted:bool = serde_json::from_value(value.clone()).map_err(StoreError::JsonError)?;
                        println!("blacklisted obtained is: {:?}", blacklisted);
                        custom_query = custom_query.bind(blacklisted);
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
                "token_type" => serde_json::json!(row.try_get::<TokenType, _>(column_name)?),
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
        let token_obj: Token = serde_json::from_value(item).map_err(StoreError::JsonError)?;

        let token = token_obj.get_token().to_string();
        println!("token to insert is: {}",token);
        let token_type = token_obj.get_type();
        let blacklisted = token_obj.get_blacklisted();
        // TODO
        // Store Hash instead of clear password, will be done after implementation of crypto library
        sqlx::query!(
            // language=PostgreSQL
            r#"
                    insert into "tokens"(token_string,token_type,blacklisted)
                    values ($1, $2, $3)"#,
            token,
            token_type as TokenType,
            blacklisted
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
        let rows = sqlx::query_as!(TokenRow, r#"SELECT id, token_string, created_at, updated_at, token_type AS "token_type!: TokenType", blacklisted FROM tokens WHERE id = $1"#, id)
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
                update tokens set token_string = $1, token_type = $2, blacklisted = $3, updated_at = $4 where id=$5"#,
            token_data.get_token(),
            token_data.get_type() as TokenType,
            token_data.get_blacklisted(),
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
        if let serde_json::Value::Object(map) = json_slug.clone() {
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
        let mut custom_query = sqlx::query(&custom_query);
        custom_query = self.bind_values(custom_query, &json_slug)?;
        // for value in values{
        //     custom_query = custom_query.bind(value);
        // }
        //print!("final query is: {:?}",custom_query);
        let rows = custom_query
            .fetch_all(connection)
            .await
            .map_err(StoreError::SqlxError)?;
        println!("should have fetched");
        let user_datas = rows
            .iter()
            .map(|row| self.row_to_json(row).map_err(StoreError::SqlxError))
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
             r#"SELECT id, token_string, created_at, updated_at, token_type AS "token_type!: TokenType", blacklisted FROM tokens order by id asc limit $1 offset $2"#,
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
        let mut custom_query: String = String::from(r#"update tokens set "#);
        let mut conditions = Vec::new();
        let mut max_variable = 0;
        // Check if the parsed value is an object
        if let serde_json::Value::Object(map) = patch.clone() {
            // Iterate over the key-value pairs in the object

            for (key, value) in map {
                println!("Key: {}, Value: {}", key, value);
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
        let affected_rows = custom_query
            .execute(connection)
            .await
            .map_err(StoreError::SqlxError)?;

        println!("Updated {} row(s)", affected_rows.rows_affected());
        Ok(())
    }
}
