use actix_web::{get, post, web, Responder};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{auth::Authority, entity::{prelude::*, user}};

pub(super) fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(login)
        .service(whoami);
}

#[derive(Debug, Serialize, Deserialize)]
struct Login {
    username: String,
    password: String,
}

#[post("/login")]
async fn login(db: web::Data<DatabaseConnection>, authority: web::Data<Authority>, credentials: web::Json<Login>) -> impl Responder {
    let hashed_password = &Sha256::digest(&format!("{}:{}", credentials.password, credentials.username))[..];
    
    let Some(user) = User::find()
        .filter(user::Column::Username.eq(&credentials.username))
        .filter(user::Column::Password.eq(hashed_password))
        .one(db.get_ref()).await.unwrap()
    else {
        return Err(actix_web::error::ErrorForbidden("invalid credentials"));
    };

    Ok(
        authority.issue_for(&user)
    )
}

#[get("")]
async fn whoami(user: user::Model) -> impl Responder {
    web::Json(user)
}

#[cfg(test)]
mod tests {
    use actix_web::{body::MessageBody, http::{Method, StatusCode}, test, App};
    use chrono::Local;
    use sea_orm::{DatabaseBackend, MockDatabase};
    use uuid::Uuid;

    use crate::entity::sea_orm_active_enums::RoleType;

    use super::*;

    #[actix_web::test]
    async fn test_login() {
        let secret = b"secret";

        let user_password = "secret";
        let user = user::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            username: "Bob".to_string(),
            password: Sha256::digest(&format!("{}:{}", user_password, "Bob")).to_vec(),
            role: RoleType::Employee,
            salary: 1_000_000,
        };

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![ ],
                vec![ user.clone() ],
            ]);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Authority::new(secret)))
                .app_data(web::Data::new(db.into_connection()))
                .service(login)
        ).await;
        
        {
            let forbidden_req = test::TestRequest::default()
                .uri("/login")
                .method(Method::POST)
                .set_json(Login {
                    username: "username".to_owned(),
                    password: "password".to_owned(),
                })
                .to_request();

            let response = test::call_service(&app, forbidden_req).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        {
            let success_req = test::TestRequest::default()
                .uri("/login")
                .method(Method::POST)
                .set_json(Login {
                    username: user.username.clone(),
                    password: user_password.to_owned(),
                })
                .to_request();

            let response = test::call_service(&app, success_req).await;
            assert_eq!(response.status(), StatusCode::OK);
            
            let body = response.into_body().try_into_bytes().unwrap();
            let returned_user = Authority::new(secret).authorize(String::from_utf8_lossy(&body)).unwrap();
            assert_eq!(returned_user, user);
        }
    }
}
