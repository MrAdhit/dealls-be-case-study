use chrono::{DateTime, FixedOffset};

use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CreateAttendance {
    pub(super) start_at: DateTimeWithTimeZone,
    pub(super) end_at: DateTimeWithTimeZone,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CreateOvertime {
    pub(super) extra_hours: i16,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CreateReimbursement {
    pub(super) description: String,
    pub(super) amount: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EmployeePayslip {
    pub(super) employee: EmployeePayslipEmployee,
    pub(super) period: EmployeePayslipPeriod,
    pub(super) attendance: EmployeePayslipAttendance,
    pub(super) overtimes: Vec<EmployeePayslipOvertime>,
    pub(super) reimbursements: Vec<EmployeePayslipReimbursement>,
    pub(super) summary: EmployeePayslipSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EmployeePayslipEmployee {
    pub(super) id: Uuid,
    pub(super) username: String,
    pub(super) base_salary: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EmployeePayslipPeriod {
    pub(super) start_at: DateTime<FixedOffset>,
    pub(super) end_at: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EmployeePayslipAttendance {
    pub(super) total_days: u64,
    pub(super) prorated_amount: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EmployeePayslipOvertime {
    pub(super) date: DateTime<FixedOffset>,
    pub(super) hours: i16,
    pub(super) amount: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EmployeePayslipReimbursement {
    pub(super) description: String,
    pub(super) amount: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EmployeePayslipSummary {
    pub(super) base_salary: i64,
    pub(super) prorated_amount: i64,
    pub(super) overtime_total: i64,
    pub(super) reimbursement_total: i64,
    pub(super) take_home_pay: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct EmployeePayslips {
    pub(super) payslips: Vec<EmployeePayslip>,
    pub(super) total_take_home: i64,
}
