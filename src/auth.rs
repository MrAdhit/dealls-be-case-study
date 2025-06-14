use std::{fmt::Debug, ops::Deref};

use actix_web::{body, dev, http::{self, header::ContentType, StatusCode}, web, FromRequest, HttpRequest, HttpResponse};
use chrono::{Duration, Local};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::entity::{sea_orm_active_enums::RoleType, user};

/// Poor man's authentication
///
/// Got no time setting up proper auth
pub struct Authority {
    jwt_key: (EncodingKey, DecodingKey),
}

impl Authority {
    pub fn new(jwt_key: &[u8]) -> Self {
        Self {
            jwt_key: (EncodingKey::from_secret(jwt_key), DecodingKey::from_secret(jwt_key))
        }
    }
    
    /// Issue a token for specified user with 1 week of expiration time
    pub fn issue_for(&self, user: &user::Model) -> String {
        let claims = Claims {
            exp: (Local::now() + Duration::weeks(1)).timestamp(),
            data: user
        };

        encode(&Header::default(), &claims, &self.jwt_key.0).unwrap()
    }
    
    pub fn authorize(&self, token: impl AsRef<str>) -> Result<user::Model, AuthError> {
        let payload = decode::<Claims<user::Model>>(token.as_ref(), &self.jwt_key.1, &Validation::default())?;
        
        Ok(payload.claims.data)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims<T> {
    exp: i64,
    data: T,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("authority error")]
    AuthorityError(#[from] jsonwebtoken::errors::Error),
}

impl actix_web::error::ResponseError for AuthError {
    fn error_response(&self) -> HttpResponse<body::BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::plaintext())
            .body(self.to_string())
    }
    
    fn status_code(&self) -> http::StatusCode {
        match self {
            AuthError::AuthorityError(_) => StatusCode::FORBIDDEN,
        }
    }
}

impl FromRequest for user::Model {
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let req = req.clone();
        
        Box::pin(async move {
            // Basically grabs the value after space ( ) from `Authorization` header
            // Example: JWT sometoken
            //              ^ grabs this value
            let Some(Ok(Some((_, token)))) = req.headers()
                .get("Authorization")
                .map(|v|
                    v.to_str()
                        .map(|str| str.split_once(" "))
                )
            else {
                return Err(actix_web::error::ErrorUnauthorized("unauthorized"))
            };
            
            let authority = req.app_data::<web::Data<Authority>>().expect("Authority must be attached");
            let user = authority.authorize(token)?;
            
            Ok(user)
        })
    }
}

pub struct Admin(pub user::Model);

impl Deref for Admin {
    type Target = user::Model;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for Admin {
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let req = req.clone();
        
        Box::pin(async move {
            let user = user::Model::from_request(&req, &mut dev::Payload::None).await?;
            
            if user.role != RoleType::Admin {
                return Err(actix_web::error::ErrorForbidden("forbidden"))
            }
    
            Ok(Self(user))
        })
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{body::MessageBody, get, test, web, App, Responder};
    use uuid::Uuid;

    use crate::entity::sea_orm_active_enums::RoleType;

    use super::*;
    
    #[actix_web::test]
    async fn test_authority() {
        let authority = Authority::new(b"secret");
        
        let user = user::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            username: "Bob".to_string(),
            password: Vec::new(),
            role: RoleType::Employee,
            salary: 1_000_000,
        };
        
        let token = authority.issue_for(&user);

        let authorized_user = authority.authorize(token).expect("Unable to authorize user from token");
        assert_eq!(user, authorized_user);
    }
    
    #[actix_web::test]
    async fn test_extractor() {
        let secret = b"secret";
        
        #[get("/")]
        async fn test_handler(user: user::Model) -> impl Responder {
            user.id.to_string()
        }
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Authority::new(secret)))
                .service(test_handler)
        ).await;
        
        {
            let forbidden_req = test::TestRequest::default()
                .uri("/")
                .insert_header(("Authorization", "JWT wrong"))
                .to_request();
            
            let response = test::call_service(&app, forbidden_req).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        {
            let unauthorized_req = test::TestRequest::default()
                .uri("/")
                .to_request();
            
            let response = test::call_service(&app, unauthorized_req).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        {
            let user = user::Model {
                id: Uuid::new_v4(),
                created_at: Local::now().into(),
                updated_at: Local::now().into(),
                username: "Bob".to_string(),
                password: Vec::new(),
                role: RoleType::Employee,
                salary: 1_000_000,
            };
    
            let token = Authority::new(secret).issue_for(&user);
    
            let authorized_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("JWT {token}")))
                .to_request();
            
            let response = test::call_service(&app, authorized_req).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.into_body().try_into_bytes().unwrap(), user.id.to_string().as_bytes());
        }
    }
    
    #[actix_web::test]
    async fn test_admin_extractor() {
        let secret = b"secret";
        
        #[get("/")]
        async fn test_handler(user: Admin) -> impl Responder {
            assert_eq!(user.role, RoleType::Admin);
            
            ""
        }
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Authority::new(secret)))
                .service(test_handler)
        ).await;
        
        {
            let user_admin = user::Model {
                id: Uuid::new_v4(),
                created_at: Local::now().into(),
                updated_at: Local::now().into(),
                username: "Bob".to_string(),
                password: Vec::new(),
                role: RoleType::Admin,
                salary: 1_000_000,
            };

            let token = Authority::new(secret).issue_for(&user_admin);

            let success_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("JWT {token}")))
                .to_request();
            
            let response = test::call_service(&app, success_req).await;
            assert_eq!(response.status(), StatusCode::OK);
        }

        {
            let user_employee = user::Model {
                id: Uuid::new_v4(),
                created_at: Local::now().into(),
                updated_at: Local::now().into(),
                username: "Bob".to_string(),
                password: Vec::new(),
                role: RoleType::Employee,
                salary: 1_000_000,
            };

            let token = Authority::new(secret).issue_for(&user_employee);

            let forbidden_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("JWT {token}")))
                .to_request();
            
            let response = test::call_service(&app, forbidden_req).await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }
    }
}
