use std::ops::Deref;

use super::*;

impl FromRequest for attendance_period::Model {
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let req = req.clone();
        
        Box::pin(async move {
            let attendance_id = req.match_info().get("attendance_id").expect("This extractor must be used under `attendance_id` path");
            let Ok(attendance_id) = Uuid::from_str(attendance_id) else {
                return Err(actix_web::error::ErrorBadRequest("invalid `attendance_id`"))
            };

            let db = req.app_data::<web::Data<DatabaseConnection>>().expect("DatabaseConnection must be attached");
            
            let Some(attendance) = AttendancePeriod::find_by_id(attendance_id)
                .one(db.as_ref()).await.unwrap()
            else {
                return Err(actix_web::error::ErrorNotFound(""))
            };

            Ok(attendance)
        })
    }
}

pub(super) struct UnprocessedAttendance(pub(super) attendance_period::Model);

impl Deref for UnprocessedAttendance {
    type Target = attendance_period::Model;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for UnprocessedAttendance {
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let req = req.clone();
        
        Box::pin(async move {
            let attendance = attendance_period::Model::from_request(&req, &mut dev::Payload::None).await?;
            
            if attendance.processed {
                return Err(actix_web::error::ErrorBadRequest("attendance is already processed"));
            }
            
            Ok(Self(attendance))
        })
    }
}

pub(super) struct ProcessedAttendance(pub(super) attendance_period::Model);

impl Deref for ProcessedAttendance {
    type Target = attendance_period::Model;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for ProcessedAttendance {
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        let req = req.clone();
        
        Box::pin(async move {
            let attendance = attendance_period::Model::from_request(&req, &mut dev::Payload::None).await?;
            
            if !attendance.processed {
                return Err(actix_web::error::ErrorBadRequest("attendance is not processed"));
            }
            
            Ok(Self(attendance))
        })
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{http::StatusCode, test, App};
    use chrono::{Duration, Local};
    use sea_orm::{DatabaseBackend, MockDatabase};

    use crate::{auth::Authority, entity::{sea_orm_active_enums::RoleType, user}};

    use super::*;

    #[actix_web::test]
    async fn test_attendance_extractor() {
        #[get("/{attendance_id}")]
        async fn test_handler(attendance: attendance_period::Model) -> impl Responder {
            web::Json(attendance)
        }

        let secret = b"secret";

        let user = user::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            username: "Bob".to_string(),
            password: Vec::new(),
            role: RoleType::Employee,
            salary: 1_000_000,
        };
        
        let attendance = attendance_period::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            created_by: Some(user.id),
            updated_by: Some(user.id),
            start_at: Local::now().into(),
            end_at: (Local::now() + Duration::days(30)).into(),
            processed: false,
        };

        let token = Authority::new(secret).issue_for(&user);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![ attendance.clone() ],
            ]);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Authority::new(secret)))
                .app_data(web::Data::new(db.into_connection()))
                .service(test_handler)
        ).await;

        let req = test::TestRequest::default()
            .uri(&format!("/{}", attendance.id.to_string()))
            .insert_header(("Authorization", format!("JWT {token}")))
            .to_request();

        let returned_attendance: attendance_period::Model = test::call_and_read_body_json(&app, req).await;
        assert_eq!(returned_attendance, attendance);
    }

    #[actix_web::test]
    async fn test_unprocessed_attendance_extractor() {
        #[get("/{attendance_id}")]
        async fn test_handler(attendance: UnprocessedAttendance) -> impl Responder {
            web::Json(attendance.0)
        }

        let secret = b"secret";

        let user = user::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            username: "Bob".to_string(),
            password: Vec::new(),
            role: RoleType::Employee,
            salary: 1_000_000,
        };
        
        let unprocessed_attendance = attendance_period::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            created_by: Some(user.id),
            updated_by: Some(user.id),
            start_at: Local::now().into(),
            end_at: (Local::now() + Duration::days(30)).into(),
            processed: false,
        };

        let processed_attendance = attendance_period::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            created_by: Some(user.id),
            updated_by: Some(user.id),
            start_at: Local::now().into(),
            end_at: (Local::now() + Duration::days(30)).into(),
            processed: true,
        };

        let token = Authority::new(secret).issue_for(&user);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![ unprocessed_attendance.clone() ],
                vec![ processed_attendance.clone() ],
            ]);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Authority::new(secret)))
                .app_data(web::Data::new(db.into_connection()))
                .service(test_handler)
        ).await;

        let req = test::TestRequest::default()
            .uri(&format!("/{}", unprocessed_attendance.id.to_string()))
            .insert_header(("Authorization", format!("JWT {token}")))
            .to_request();

        let returned_attendance: attendance_period::Model = test::call_and_read_body_json(&app, req).await;
        assert_eq!(returned_attendance, unprocessed_attendance);

        let req = test::TestRequest::default()
            .uri(&format!("/{}", processed_attendance.id.to_string()))
            .insert_header(("Authorization", format!("JWT {token}")))
            .to_request();
        
        let response = test::call_service(&app, req).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_processed_attendance_extractor() {
        #[get("/{attendance_id}")]
        async fn test_handler(attendance: ProcessedAttendance) -> impl Responder {
            web::Json(attendance.0)
        }

        let secret = b"secret";

        let user = user::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            username: "Bob".to_string(),
            password: Vec::new(),
            role: RoleType::Employee,
            salary: 1_000_000,
        };
        
        let unprocessed_attendance = attendance_period::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            created_by: Some(user.id),
            updated_by: Some(user.id),
            start_at: Local::now().into(),
            end_at: (Local::now() + Duration::days(30)).into(),
            processed: false,
        };

        let processed_attendance = attendance_period::Model {
            id: Uuid::new_v4(),
            created_at: Local::now().into(),
            updated_at: Local::now().into(),
            created_by: Some(user.id),
            updated_by: Some(user.id),
            start_at: Local::now().into(),
            end_at: (Local::now() + Duration::days(30)).into(),
            processed: true,
        };

        let token = Authority::new(secret).issue_for(&user);

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([
                vec![ processed_attendance.clone() ],
                vec![ unprocessed_attendance.clone() ],
            ]);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Authority::new(secret)))
                .app_data(web::Data::new(db.into_connection()))
                .service(test_handler)
        ).await;

        let req = test::TestRequest::default()
            .uri(&format!("/{}", processed_attendance.id.to_string()))
            .insert_header(("Authorization", format!("JWT {token}")))
            .to_request();

        let returned_attendance: attendance_period::Model = test::call_and_read_body_json(&app, req).await;
        assert_eq!(returned_attendance, processed_attendance);

        let req = test::TestRequest::default()
            .uri(&format!("/{}", unprocessed_attendance.id.to_string()))
            .insert_header(("Authorization", format!("JWT {token}")))
            .to_request();
        
        let response = test::call_service(&app, req).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
