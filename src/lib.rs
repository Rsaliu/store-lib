pub mod stores;
use crate::stores::{store::Store,user_store::UserPGStore,store::StoreError,user_store::UserRow};
use user_lib::user::user::{User,UserRoles};
//use token_lib::token::token::Token;

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use stores::{store::StoreTrait, token_store::{TokenPGStore, TokenRow}};
    use random_string::generate;
    use user_lib::user;
    use sqlx::{postgres::PgPoolOptions, Postgres,Pool};
    use crypto_lib::crypto::crypto::CryptoOp;
    use chrono::{Utc,NaiveDateTime};
    use uuid::Uuid;
    use std::env;
    use token_lib::token::token::{Token,TokenType};
    use super::*;

    async fn get_connection(db_url:&str)-> Result<Pool<Postgres>, Box<dyn std::error::Error>>
    {
        let db = match PgPoolOptions::new()
            .max_connections(10)
            .connect(db_url)
            .await
        {
            Ok(pool) => {
                log::info!("Connection to the database is successful!");
                pool
            }
            Err(err) => {
                log::error!("Failed to connect to the database: {:?}", err);
                std::process::exit(1);
            }
        };
        Ok(db)
    }
    fn get_random_string(length: usize)->String{
        let charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        generate(length, charset)
    }
    fn get_sample_user()->User{
        let mString = get_random_string(10);
        println!("Generated username is: {}",mString);
        let email = format!("{}@gmail.com",mString);
        User::new(mString,String::from("rillo"),email,UserRoles::Normal)
    }

    #[tokio::test]
    async fn user_pg_test() {

        let db_url:String = String::from("postgres://postgres@localhost/test_db");
        let db_connection:Pool<Postgres>  = get_connection(&db_url).await.expect("could not acquire connection");

        //conection
        let user_store = UserPGStore::default();
        let dummy_user = get_sample_user().clone();
        let user_json = serde_json::to_value(&dummy_user).expect("serialization failed");

        // insertion
        user_store.insert(&db_connection,user_json).await.expect("insertion failed");
        println!("insertion successful");

        let count = user_store.count(&db_connection).await.expect("get count failed");
        println!("initial count is: {count}");

        let dummy_user = get_sample_user().clone();
        let user_json = serde_json::to_value(&dummy_user).expect("serialization failed");

        // insertion
        user_store.insert(&db_connection,user_json).await.expect("insertion failed");
        println!("insertion successful");

        let new_count = user_store.count(&db_connection).await.expect("get count failed");
        println!("new count is: {new_count}");

        assert_eq!(new_count,count+1);



        //selection
        let user_data = user_store.get_by_username(&db_connection,dummy_user.get_name()).await.expect("unable to get user with name");
        let user_row:UserRow = serde_json::from_value(user_data).expect("json conversion error");
        let returned_data:User=user_row.into();
        assert_eq!(dummy_user.get_name(),returned_data.get_name());
        assert_eq!(dummy_user.get_role(),returned_data.get_role());


        //pagination test
        let m_limit = 5;
        let user_data_vec = user_store.get_all_paginate(&db_connection,m_limit,0).await.expect("unable to get paginated user");

        assert!(user_data_vec.len() as i64 <= m_limit);

        
        //update
        let id = returned_data.get_id();
        let new_user = get_sample_user().clone();
        let new_user_json = serde_json::to_value(new_user.clone()).expect("Json conversion failed");
        user_store.update(&db_connection,id, new_user_json).await.expect("user update failed");
        let user_data = user_store.get(&db_connection,id).await.expect("unable to get user with id");
        let muser = user_data.first().unwrap().to_owned();
        let user_row:UserRow = serde_json::from_value(muser).expect("json conversion error");
        let returned_data:User=user_row.into();
        assert_eq!(new_user.get_name(),returned_data.get_name());
        assert_eq!(new_user.get_role(),returned_data.get_role());

        //get by slug
        let json_slug = serde_json::json!({
            "username": new_user.get_name()
        });
        
        let user_data = user_store.get_by_slug(&db_connection,json_slug).await.expect("unable to get user with name");
        let muser = user_data.first().unwrap().to_owned();
        let user_row:UserRow = serde_json::from_value(muser).expect("json conversion error");
        let returned_data:User=user_row.into();
        assert_eq!(new_user.get_name(),returned_data.get_name());
        assert_eq!(new_user.get_role(),returned_data.get_role());

        let user_data = user_store.get(&db_connection,returned_data.get_id()).await.expect("unable to get user with id");
        let muser = user_data.first().unwrap().to_owned();
        let user_row:UserRow = serde_json::from_value(muser).expect("json conversion error");
        assert_eq!(new_user.get_name(),user_row.username);
        // patch
        let json_patch = serde_json::json!({
            "username": get_random_string(10)
        });
        println!("returned data id is: {}",returned_data.get_id());
        user_store.patch(&db_connection,returned_data.get_id(),json_patch.clone()).await.expect("unable to patch user");
        let user_data = user_store.get(&db_connection,returned_data.get_id()).await.expect("unable to get user with name");

        let user_data = user_data.first().unwrap();

        let user_row:UserRow = serde_json::from_value(user_data.to_owned()).expect("json conversion error");
        let json_val = json_patch.get("username").and_then(serde_json::Value::as_str).unwrap();
        assert_eq!(user_row.username,json_val.to_owned());
        //delete
        user_store.delete(&db_connection,id).await.expect("delete by id failed");
    }

    #[tokio::test]
    async fn token_pg_test() {

        let db_url:String = String::from("postgres://postgres@localhost/test_db");
        let db_connection:Pool<Postgres>  = get_connection(&db_url).await.expect("could not acquire connection");

        //conection
        let token_store = TokenPGStore::default();

        dotenvy::from_path(".env").expect("dot env error");
        let key = env::var("HMAC_KEY").expect("env variable error");
        let user_id = uuid::Uuid::new_v4();
        let expiry = Utc::now().naive_utc();
        let user_id_string= user_id.to_string();
        let token_header_data = serde_json::json!({"user_id":user_id_string.as_str(),"timestamp":expiry.to_string()}).to_string();
        let token_string = CryptoOp::default().generate_token(&key, token_header_data.clone()).await.expect("token geenration failure");
        let new_token = Token::new(
            user_id,
            token_string,
            expiry,
            TokenType::AccessToken
        );

        let token_json = serde_json::to_value(&new_token).expect("serialization failed");

        // insertion
        token_store.insert(&db_connection,token_json).await.expect("insertion failed");
        println!("insertion successful");

        let count = token_store.count(&db_connection).await.expect("get count failed");
        println!("initial count is: {count}");

        let second_user_id = uuid::Uuid::new_v4();
        let expiry = Utc::now().naive_utc();
        let second_user_id_string= second_user_id.to_string();
        let second_token_string = CryptoOp::default().generate_token(&key, second_user_id_string).await.expect("token geenration failure");
        let second_new_token = Token::new(
            second_user_id,
            second_token_string,
            expiry,
            TokenType::RefreshToken
        );
        let second_token_json = serde_json::to_value(&second_new_token).expect("serialization failed");
        // insertion
        token_store.insert(&db_connection,second_token_json).await.expect("insertion failed");
        println!("insertion successful");

        let new_count = token_store.count(&db_connection).await.expect("get count failed");
        println!("new count is: {new_count}");

        assert_eq!(new_count,count+1);



        //selection
        let mut m_id:Uuid= Uuid::nil();
        let token_data = token_store.get_by_userid(&db_connection,second_user_id).await.expect("unable to get user with name");
        if token_data.len() > 0 {
            let token_row = token_data.first().unwrap();
            let token_row:TokenRow = serde_json::from_value(token_row.to_owned()).expect("Json conversion error");
            let mtoken:Token = token_row.into();
            println!("mtoken: {:?} second_token: {:?}",mtoken,second_new_token);
            assert_eq!(mtoken.get_expired_time().format("%Y:%m:%d %H:%M").to_string(),second_new_token.get_expired_time().format("%Y:%m:%d %H:%M").to_string());
            assert_eq!(mtoken.get_token(),second_new_token.get_token());
            assert_eq!(mtoken.get_user_id(),second_new_token.get_user_id());
            m_id = mtoken.get_id();
        }



        //pagination test
        let m_limit = 5;
        let token_data_vec = token_store.get_all_paginate(&db_connection,m_limit,0).await.expect("unable to get paginated token");

        assert!(token_data_vec.len() as i64 <= m_limit);

        
        //update
        let id = m_id;
        println!("new token_id is: {:?}",id);
        let expiry = Utc::now().naive_utc();
        let token_header_data = serde_json::json!({"user_id":id.to_string(),"timestamp":expiry.to_string()}).to_string();
        let dummy_token = CryptoOp::default().generate_token(&key, token_header_data.clone()).await.expect("token geenration failure");
        let third_new_token = Token::new_full(
            id,
            dummy_token.clone(),
            new_token.get_user_id(),
            new_token.get_expired_time(),
            new_token.get_type(),
            new_token.get_blacklisted()
        );
        let third_token_json = serde_json::to_value(&third_new_token).expect("serialization failed");
        println!("thrid token json {:?}",third_token_json);
        // insertion
        token_store.update(&db_connection,id,third_token_json).await.expect("update failed");
        println!("update successful");
        let retuned_token = token_store.get(&db_connection, id).await.expect("unable to get token");
        println!("returned token details are: {:?}",retuned_token);
        let returned_token =  retuned_token.first().unwrap();
        let returned_token:TokenRow = serde_json::from_value(returned_token.to_owned()).expect("json conversion error");
        let returned_token:Token = returned_token.into();
        assert_eq!(dummy_token,returned_token.get_token());

        //get by slug
        let json_slug = serde_json::json!({
            "user_id": returned_token.get_user_id().to_string()
        });
        
        let token_data = token_store.get_by_slug(&db_connection,json_slug).await.expect("unable to get user with name");
        println!("slug returned: {:?}",token_data);
        let mtoken = token_data.last().unwrap().to_owned();
        let mtoken_row:TokenRow = serde_json::from_value(mtoken).expect("json conversion error");
        let returned_data:Token=mtoken_row.into();
        assert_eq!(returned_data.get_id(),id);


        let json_slug = serde_json::json!({
            "token_string": returned_token.get_token().to_string()
        });
        
        let token_data = token_store.get_by_slug(&db_connection,json_slug).await.expect("unable to get user with name");
        println!("slug returned: {:?}",token_data);
        let mtoken = token_data.last().unwrap().to_owned();
        let mtoken_row:TokenRow = serde_json::from_value(mtoken).expect("json conversion error");
        let returned_data:Token=mtoken_row.into();
        assert_eq!(returned_data.get_id(),id);

        // patch
        let patch_token = CryptoOp::default().generate_token(&key, token_header_data.clone()).await.expect("token geenration failure");
        let json_patch = serde_json::json!({
            "token_string": patch_token 
        });
        println!("returned data id is: {}",returned_data.get_id());
        token_store.patch(&db_connection,returned_data.get_id(),json_patch.clone()).await.expect("unable to patch user");
        let token_data = token_store.get(&db_connection,returned_data.get_id()).await.expect("unable to get user with name");

        let token_data = token_data.first().unwrap();

        let token_row:TokenRow = serde_json::from_value(token_data.to_owned()).expect("json conversion error");
        let json_val = json_patch.get("token_string").and_then(serde_json::Value::as_str).unwrap();
        assert_eq!(token_row.token_string.trim_matches('"').to_string(),json_val.to_owned());


        //delete
        token_store.delete(&db_connection,id).await.expect("delete by id failed");
    }
}
