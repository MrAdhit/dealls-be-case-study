use std::str::FromStr;

use actix_web::{dev, get, post, web, FromRequest, HttpRequest, HttpResponse, Responder};
use chrono::{Datelike, Local, Timelike, Utc, Weekday};
use futures_util::future::LocalBoxFuture;
use sea_orm::{prelude::DateTimeWithTimeZone, ActiveValue::{Set, Unchanged}, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{auth::Admin, consts::{self, WORKING_HOUR}, entity::{attendance_period, employee_attendance, employee_overtime, employee_reimbursement, prelude::*, sea_orm_active_enums::RoleType, user}, pages::attendance::extractor::{ProcessedAttendance, UnprocessedAttendance}, utils};

use model::*;

mod extractor;
mod model;

pub(super) fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(create_attendance)
        .service(get_attendance)
        .service(create_employee_attendance)
        .service(create_employee_overtime)
        .service(create_employee_reimbursement)
        .service(process_payroll)
        .service(get_payslip)
        .service(get_employee_payslips);
}

#[post("")]
async fn create_attendance(db: web::Data<DatabaseConnection>, admin: Admin, payload: web::Json<CreateAttendance>) -> impl Responder {
    if payload.end_at < payload.start_at {
        return Err(actix_web::error::ErrorBadRequest("end_at is lower than start_at"))
    }

    let attendance = attendance_period::ActiveModel {
        created_by: Set(Some(admin.id)),
        updated_by: Set(Some(admin.id)),
        created_at: Set(Local::now().fixed_offset()),
        updated_at: Set(Local::now().fixed_offset()),
        start_at: Set(payload.start_at),
        end_at: Set(payload.end_at),
        ..Default::default()
    };
    
    let res = AttendancePeriod::insert(attendance)
        .exec_with_returning(db.as_ref()).await.unwrap();

    Ok(
        HttpResponse::Created()
            .json(web::Json(res))
    )
}

#[get("/{attendance_id}")]
async fn get_attendance(attendance: attendance_period::Model) -> impl Responder {
    web::Json(attendance)
}

#[post("/{attendance_id}")]
async fn create_employee_attendance(db: web::Data<DatabaseConnection>, employee: user::Model, attendance: UnprocessedAttendance) -> impl Responder {
    let now = Utc::now().with_timezone(&attendance.created_at.timezone());

    // So glad I use Rust
    if let Weekday::Sat | Weekday::Sun = now.weekday() {
        return Err(actix_web::error::ErrorBadRequest("cannot attend on weekend"));
    }

    let (start_of_day, end_of_day) = utils::get_today_range(&now);

    let e_attendance = EmployeeAttendance::find()
        .filter(employee_attendance::Column::CreatedAt.between(start_of_day, end_of_day))
        .filter(employee_attendance::Column::CreatedBy.eq(employee.id))
        .filter(employee_attendance::Column::AttendancePeriodId.eq(attendance.id))
        .one(db.as_ref()).await.unwrap();
    
    if let Some(e_attendance) = e_attendance {
        return Ok(HttpResponse::Ok().json(web::Json(e_attendance)))
    }

    let model = employee_attendance::ActiveModel {
        attendance_period_id: Set(attendance.id),
        created_by: Set(Some(employee.id)),
        updated_by: Set(Some(employee.id)),
        created_at: Set(Local::now().fixed_offset()),
        updated_at: Set(Local::now().fixed_offset()),
        ..Default::default()
    };
    
    let e_attendance = EmployeeAttendance::insert(model)
        .exec_with_returning(db.as_ref()).await.unwrap();
    
    Ok(HttpResponse::Created()
        .json(web::Json(e_attendance)))
}

#[post("/{attendance_id}/overtime")]
async fn create_employee_overtime(db: web::Data<DatabaseConnection>, employee: user::Model, attendance: UnprocessedAttendance, payload: web::Json<CreateOvertime>) -> impl Responder {
    let now = Utc::now().with_timezone(&attendance.created_at.timezone());

    let (start_of_day, end_of_day) = utils::get_today_range(&now);
    
    let Some(_) = EmployeeAttendance::find()
        .filter(employee_attendance::Column::CreatedAt.between(start_of_day, end_of_day))
        .filter(employee_attendance::Column::CreatedBy.eq(employee.id))
        .filter(employee_attendance::Column::AttendancePeriodId.eq(attendance.id))
        .one(db.as_ref()).await.unwrap()
    else {
        return Err(actix_web::error::ErrorBadRequest("you have not checked-in today"))
    };
    
    if now.hour() < consts::WORKING_HOUR.1 {
        return Err(actix_web::error::ErrorBadRequest("your work hours are not done yet"))
    };
    
    let existing_e_overtime = EmployeeOvertime::find()
        .filter(employee_overtime::Column::CreatedAt.between(start_of_day, end_of_day))
        .filter(employee_overtime::Column::CreatedBy.eq(employee.id))
        .filter(employee_overtime::Column::AttendancePeriodId.eq(attendance.id))
        .one(db.as_ref()).await.unwrap();
    
    match existing_e_overtime {
        Some(overtime) => {
            let new_hours = overtime.extra_hours + payload.extra_hours;
            if new_hours > 3 {
                return Err(actix_web::error::ErrorBadRequest("you cannot take overtime for more than 3 hours a day"))
            }
            
            let model = EmployeeOvertime::update(employee_overtime::ActiveModel {
                id: Unchanged(overtime.id),
                updated_at: Set(Local::now().fixed_offset()),
                updated_by: Set(Some(employee.id)),
                extra_hours: Set(new_hours),
                ..Default::default()
            }).exec(db.as_ref()).await.unwrap();
            
            Ok(HttpResponse::Ok().json(web::Json(model)))
        },
        None => {
            if payload.extra_hours > 3 {
                return Err(actix_web::error::ErrorBadRequest("you cannot take overtime for more than 3 hours a day"))
            }

            let model = EmployeeOvertime::insert(employee_overtime::ActiveModel {
                created_by: Set(Some(employee.id)),
                updated_by: Set(Some(employee.id)),
                created_at: Set(Local::now().fixed_offset()),
                updated_at: Set(Local::now().fixed_offset()),
                extra_hours: Set(payload.extra_hours),
                attendance_period_id: Set(attendance.id),
                ..Default::default()
            }).exec_with_returning(db.as_ref()).await.unwrap();
            
            Ok(HttpResponse::Created().json(web::Json(model)))
        },
    }
}

#[post("/{attendance_id}/reimburse")]
async fn create_employee_reimbursement(db: web::Data<DatabaseConnection>, employee: user::Model, attendance: UnprocessedAttendance, payload: web::Json<CreateReimbursement>) -> impl Responder {
    let model = EmployeeReimbursement::insert(employee_reimbursement::ActiveModel {
        created_by: Set(Some(employee.id)),
        updated_by: Set(Some(employee.id)),
        created_at: Set(Local::now().fixed_offset()),
        updated_at: Set(Local::now().fixed_offset()),
        description: Set(payload.description.clone()),
        amount: Set(payload.amount),
        attendance_period_id: Set(attendance.id),
        ..Default::default()
    }).exec_with_returning(db.as_ref()).await.unwrap();

    HttpResponse::Created().json(web::Json(model))
}

#[post("/{attendance_id}/process_payroll")]
async fn process_payroll(db: web::Data<DatabaseConnection>, admin: Admin, attendance: UnprocessedAttendance) -> impl Responder {
    let model = AttendancePeriod::update(attendance_period::ActiveModel {
        id: Unchanged(attendance.id),
        processed: Set(true),
        updated_by: Set(Some(admin.id)),
        updated_at: Set(Local::now().fixed_offset()),
        ..Default::default()
    }).exec(db.as_ref()).await.unwrap();

    HttpResponse::Ok().json(web::Json(model))
}

async fn generate_employee_payslip(
    db: &DatabaseConnection,
    employee: user::Model,
    attendance: &ProcessedAttendance,
) -> EmployeePayslip {
    let attendance_days = EmployeeAttendance::find()
        .filter(employee_attendance::Column::AttendancePeriodId.eq(attendance.id))
        .filter(employee_attendance::Column::CreatedBy.eq(employee.id))
        .count(db).await.unwrap();

    let overtimes = EmployeeOvertime::find()
        .filter(employee_overtime::Column::AttendancePeriodId.eq(attendance.id))
        .filter(employee_overtime::Column::CreatedBy.eq(employee.id))
        .all(db).await.unwrap();
    
    let reimbursements = EmployeeReimbursement::find()
        .filter(employee_reimbursement::Column::AttendancePeriodId.eq(attendance.id))
        .filter(employee_reimbursement::Column::CreatedBy.eq(employee.id))
        .all(db).await.unwrap();
    
    let total_working_days = utils::count_working_days(attendance.start_at, attendance.end_at);

    let hourly_rate = employee.salary / ((total_working_days * (WORKING_HOUR.1 - WORKING_HOUR.0) as i64));
    let overtime_rate = hourly_rate * 2;
    
    let res_overtimes = overtimes.into_iter().map(|overtime|
        EmployeePayslipOvertime {
            date: overtime.updated_at,
            hours: overtime.extra_hours,
            amount: (overtime_rate * overtime.extra_hours as i64),
        }
    ).collect::<Vec<_>>();
    
    let res_reimbursements = reimbursements.into_iter().map(|reimbursement| 
        EmployeePayslipReimbursement {
            description: reimbursement.description,
            amount: reimbursement.amount,
        }
    ).collect::<Vec<_>>();
    
    let overtime_total = res_overtimes.iter().map(|o| o.amount).reduce(|a, b| a + b).unwrap_or_default();
    let reimbursement_total = res_reimbursements.iter().map(|r| r.amount).reduce(|a, b| a + b).unwrap_or_default();
    let prorated_amount = (employee.salary * attendance_days as i64) / total_working_days;

    EmployeePayslip {
        employee: EmployeePayslipEmployee {
            id: employee.id,
            username: employee.username,
            base_salary: employee.salary,
        },
        period: EmployeePayslipPeriod {
            start_at: attendance.start_at,
            end_at: attendance.end_at,
        },
        attendance: EmployeePayslipAttendance {
            total_days: attendance_days,
            prorated_amount,
        },
        overtimes: res_overtimes,
        reimbursements: res_reimbursements,
        summary: EmployeePayslipSummary {
            base_salary: employee.salary,
            prorated_amount,
            overtime_total,
            reimbursement_total,
            take_home_pay: prorated_amount + overtime_total + reimbursement_total,
        },
    }
}

#[get("/{attendance_id}/payslip")]
async fn get_payslip(db: web::Data<DatabaseConnection>, employee: user::Model, attendance: ProcessedAttendance) -> impl Responder {
    let payslip = generate_employee_payslip(&db, employee, &attendance).await;
    web::Json(payslip)
}

#[get("/{attendance_id}/employee_payslips")]
async fn get_employee_payslips(db: web::Data<DatabaseConnection>, _admin: Admin, attendance: ProcessedAttendance) -> impl Responder {
    let employees = User::find()
        .filter(user::Column::Role.eq(RoleType::Employee))
        .all(db.as_ref()).await.unwrap();
    
    let payslips = futures_util::future::join_all(
        employees.into_iter().map(|employee|
            generate_employee_payslip(&db, employee, &attendance)
        )
    ).await;

    web::Json(
        EmployeePayslips {
            total_take_home: payslips.iter().map(|p| p.summary.take_home_pay).reduce(|a, b| a + b).unwrap_or_default(),
            payslips,
        }
    )
}
