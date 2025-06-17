# Introduction

I made this app with Rust for the sake of performance and easy scalability (because of requirements...).\
Why easy to scale? because the app itself is stateless, so to scale, you just need to set-up multiple micro-servers (yes micro-servers, this app uses little-to-none system resource, you could even get away with using AWS free-tier VPS) and routes the traffic through a load-balancing proxy like nginx, HAProxy, etc etc.

This app handles timezone correctly by using the Rust time library/crates [chrono](https://crates.io/crates/chrono); For example, each time an employee checks-in/attend a work day, the app derives today with timezone from the attendance period (see `src/pages/attendance.rs:55` for how it handles it)

All endpoints are fully integrated-tested with a real database, see `test_integration.py` the actual tests starts from line 176.\
Every middleware and function are fully unit-tested too.

I use pytest so that it's more flexible with how I do the test; Why? because this app is time sensitive, and there is **NO** way of mocking the time without doing some _hacky_ approach.\
So to mock the time, I use [libfaketime](https://github.com/wolfcw/libfaketime) to mock the native system-time library,\
and utilizes Docker for easy & programmable reproducibility, see `test_integration.py:69` for how I make the Docker test container.\
But the catch is, it took forever to do the integration-test; But hey, still better than no testing right?

And lastly, tracing; Because I use Rust, it's actually very easy to implement the tracing.\
How? I use Tokio's [Tracing](https://crates.io/crates/tracing) to do the tracing, you could plug the library into something like [OpenTelemetry](https://opentelemetry.io/) to _beatifully_ displays the data.\
Heck, I personally use [DataDog](https://www.datadoghq.com/) thru [OpenTelemetry](https://opentelemetry.io/) with Tokio's [Tracing](https://crates.io/crates/tracing), because it's just that _good_.\
Set `RUST_LOG` env to `trace` to see every traced things.

# Getting started

- Install [Rust](https://rustup.rs/)
- Setup PostgreSQL & put the url to `.env`
- Then, go to `migration` directory and run `cargo run` to run the migrator, see `migration/README.md` for more info.
- After that, everything should be set-up correctly, so go back to the project directory and run `cargo run --release` or `cargo build --release` to just build the app.
- And lastly, the app should be accessible thru the `HOST_ADDRESS` that is set in the `.env` file

The users will also get automatically generated, there will be 100 employees and 1 admin.\
The credentials for the generated employees are 1 to 100, like (username: 1, password: 1), (username: 2, password: 2), and so on until 100.\
For admin, the credentials is (username: admin, password: admin)

# Testing

### Unit-test

- Run `cargo test`

### Integration-test

Before doing integration-test, make sure you already has Docker set-up.\
Also make sure that there's network called `bridge` in Docker, this should be automatically created by Docker.

- Install Python 3.12.6
- Optional: It is recomended to set-up Python virtual environment to run the test
- Run `pip install -r ./test_requirements.txt` to install all the test dependencies.
- Run `pytest -s` to fully integrated-test, this may take some time for the first run.

# Tracing

All traces are saved to `trace.log` file.

# System Usages

These are taken from `docker stats` while running the app with Docker.\
By default the app tries to utilize all of the cpu threads if necessary, 100% = 1 thread.

### Idling

```log
CONTAINER ID   NAME               CPU %     MEM USAGE / LIMIT     MEM %     NET I/O          BLOCK I/O   PIDS
fbba1872b923   eager_archimedes   0.10%     18.86MiB / 30.85GiB   0.06%     2.8kB / 1.31kB   0B / 0B     27
```

### Benchmarked using `rewrk`

The benchmark was run on `AMD Ryzen 9 7900X`

```log
CONTAINER ID   NAME               CPU %     MEM USAGE / LIMIT     MEM %     NET I/O         BLOCK I/O   PIDS
fbba1872b923   eager_archimedes   359.24%   20.16MiB / 30.85GiB   0.06%     207MB / 320MB   0B / 0B     27
```

Requesting to `/auth` which mean the app actively tries to validate requester token.

```log
$ rewrk -c 256 -t 12 -d 30s -h http://127.0.0.1:8080/auth --pct
Beginning round 1...
Benchmarking 256 connections @ http://127.0.0.1:8080/auth for 30 second(s)
  Latencies:
    Avg      Stdev    Min      Max
    4.34ms   1.49ms   0.47ms   199.15ms
  Requests:
    Total: 1763637 Req/Sec: 58788.67
  Transfer:
    Total: 233.79 MB Transfer Rate: 7.79 MB/Sec
+ --------------- + --------------- +
|   Percentile    |   Avg Latency   |
+ --------------- + --------------- +
|      99.9%      |     18.05ms     |
|       99%       |     9.78ms      |
|       95%       |     7.56ms      |
|       90%       |     6.74ms      |
|       75%       |     5.83ms      |
|       50%       |     5.21ms      |
+ --------------- + --------------- +
```

# API Usages

Keep in mind that the URL _are sensitive_, as in it must be written exactly as is, no extra `/`.\
See `test_integration.py` to see how these get used exactly.

Any response that is type of `application/json` are guaranteed to be the valid object data.\
Every user-error is in type of `text/plain`, containing the short error description.

### Auth

<details>
 <summary><code>POST</code> <code><b>/auth/login</b></code> <code>(Login with the provided credentials)</code></summary>

##### JSON Payload

> ```json
> {
>   "username": <Username>,
>   "password": <Plain Password> // For the sake of simplicity, the password is hashed on the server; In a perfect world, the frontend would do it
> }
> ```

##### Responses

> | http code | content-type               | response              |
> | --------- | -------------------------- | --------------------- |
> | `200`     | `text/plain;charset=UTF-8` | `jwt.access.token`    |
> | `403`     | `text/plain;charset=UTF-8` | `invalid credentials` |

</details>

<details>
 <summary><code>GET</code> <code><b>/auth</b></code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {jwt.access.token}",
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                                        | reason                                       |
> | --------- | -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
> | `200`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'username': <Username>, 'password': <Hashed Password>, 'salary': <Salary>, 'role': <Role>}` | None                                         |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                                  | Invalid/No provided `Authorization` header   |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                               | Provided `Authorization` header were invalid |

</details>

### Attendance

<details>
 <summary><code>POST</code> <code><b>/attendance</b></code> <code>(Admin only, creates a new attendace period)</code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {admin jwt.access.token}",
> }
> ```

##### JSON Payload

> ```json
> {
>   "start_at": <Datetime>,
>   "end_at": <Datetime>
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                                                               | reason                                       |
> | --------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
> | `200`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'created_by': <UUID>, 'updated_by': <UUID>, 'start_at': <Datetime>, 'end_at': <Datetime>, 'processed': <Boolean>}` | None                                         |
> | `400`     | `text/plain;charset=UTF-8` | `end_at is lower than start_at`                                                                                                                                                        | End at is lower than start at                |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                                                         | Invalid/No provided `Authorization` header   |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                                                      | Provided `Authorization` header were invalid |
> | `403`     | `text/plain;charset=UTF-8` | `forbidden`                                                                                                                                                                            | You are not allowed to access this endpoint  |

</details>

<details>
 <summary><code>GET</code> <code><b>/attendance/{attendance_id}</b></code> <code>(Get attendance period info)</code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {employee jwt.access.token}",
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                                                               | reason                                       |
> | --------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
> | `200`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'created_by': <UUID>, 'updated_by': <UUID>, 'start_at': <Datetime>, 'end_at': <Datetime>, 'processed': <Boolean>}` | None                                         |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                                                         | Invalid/No provided `Authorization` header   |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                                                      | Provided `Authorization` header were invalid |

</details>

<details>
 <summary><code>POST</code> <code><b>/attendance/{attendance_id}</b></code> <code>(Create employee attendance / employee check-in)</code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {employee jwt.access.token}",
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                         | reason                                                                                    |
> | --------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
> | `201`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'created_by': <UUID>, 'updated_by': <UUID>, 'attendance_period_id': <UUID>}` | Successfully attends/check-in for today                                                   |
> | `200`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'created_by': <UUID>, 'updated_by': <UUID>, 'attendance_period_id': <UUID>}` | Employee already attends/check-in for today, returns the attendance object and do nothing |
> | `400`     | `text/plain;charset=UTF-8` | `cannot attend on weekend`                                                                                                                       | Today's weekend, employee cannot attend/check-in                                          |
> | `400`     | `text/plain;charset=UTF-8` | `attendance is already processed`                                                                                                                | Attendance period is already processed                                                    |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                   | Invalid/No provided `Authorization` header                                                |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                | Provided `Authorization` header were invalid                                              |

</details>

<details>
 <summary><code>POST</code> <code><b>/attendance/{attendance_id}/overtime</b></code> <code>(Create employee overtime)</code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {employee jwt.access.token}",
> }
> ```

##### JSON Payload

> ```json
> {
>   "extra_hours": <Number>
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                                                  | reason                                               |
> | --------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------- |
> | `201`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'created_by': <UUID>, 'updated_by': <UUID>, 'extra_hours': <Number>, 'attendance_period_id': <UUID>}` | Successfully requested for extra hours               |
> | `200`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'created_by': <UUID>, 'updated_by': <UUID>, 'extra_hours': <Number>, 'attendance_period_id': <UUID>}` | Successfully updates employee extra hours            |
> | `400`     | `text/plain;charset=UTF-8` | `you have not checked-in today`                                                                                                                                           | Employee have not attend/check-in today              |
> | `400`     | `text/plain;charset=UTF-8` | `your work hours are not done yet`                                                                                                                                        | Employee is still in work hours (9-5)                |
> | `400`     | `text/plain;charset=UTF-8` | `you cannot take overtime for more than 3 hours a day`                                                                                                                    | Employee requested extra hours for more than 3 hours |
> | `400`     | `text/plain;charset=UTF-8` | `attendance is already processed`                                                                                                                                         | Attendance period is already processed               |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                                            | Invalid/No provided `Authorization` header           |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                                         | Provided `Authorization` header were invalid         |

</details>

<details>
 <summary><code>POST</code> <code><b>/attendance/{attendance_id}/reimburse</b></code> <code>(Create employee reimbursement)</code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {employee jwt.access.token}",
> }
> ```

##### JSON Payload

> ```json
> {
>   "description": <String, reimburse description>
>   "amount": <Number, desired amount>
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                                                                              | reason                                       |
> | --------- | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
> | `200`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'created_by': <Datetime>, 'updated_by': <Datetime>, 'amount': <Number>, 'description': <String>, 'attendance_period_id': <UUID>}` | None                                         |
> | `400`     | `text/plain;charset=UTF-8` | `attendance is already processed`                                                                                                                                                                     | Attendance period is already processed       |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                                                                        | Invalid/No provided `Authorization` header   |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                                                                     | Provided `Authorization` header were invalid |

</details>

<details>
 <summary><code>POST</code> <code><b>/attendance/{attendance_id}/process_payroll</b></code> <code>(Admin only, process payroll and lock the attendance period)</code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {admin jwt.access.token}",
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                                                                     | reason                                       |
> | --------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
> | `200`     | `application/json`         | `{'id': <UUID>, 'created_at': <Datetime>, 'updated_at': <Datetime>, 'created_by': <UUID>, 'updated_by': <UUID>, 'start_at': <Datetime>, 'end_at': <Datetime>, 'processed': <Boolean(True)>}` | None                                         |
> | `400`     | `text/plain;charset=UTF-8` | `attendance is already processed`                                                                                                                                                            | Attendance period is already processed       |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                                                               | Invalid/No provided `Authorization` header   |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                                                            | Provided `Authorization` header were invalid |
> | `403`     | `text/plain;charset=UTF-8` | `forbidden`                                                                                                                                                                                  | You are not allowed to access this endpoint  |

</details>

<details>
 <summary><code>GET</code> <code><b>/attendance/{attendance_id}/payslip</b></code> <code>(Get employee payslip)</code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {employee jwt.access.token}",
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | reason                                       |
> | --------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
> | `200`     | `application/json`         | `{'employee': {'id': <UUID>, 'username': <String>, 'base_salary': <Number>}, 'period': {'start_at': <Datetime>, 'end_at': <Datetime>}, 'attendance': {'total_days': <Number>, 'prorated_amount': <Number>}, 'overtimes': [{'date': <Datetime>, 'hours': <Number>, 'amount': <Number>}, ...], 'reimbursements': [{'description': <String>, 'amount': <Number>}, ...], 'summary': {'base_salary': <Number>, 'prorated_amount': <Number>, 'overtime_total': <Number>, 'reimbursement_total': <Number>, 'take_home_pay': <Number>}}` | None                                         |
> | `400`     | `text/plain;charset=UTF-8` | `attendance is not processed`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Attendance period has not been processed     |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Invalid/No provided `Authorization` header   |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | Provided `Authorization` header were invalid |

</details>

<details>
 <summary><code>GET</code> <code><b>/attendance/{attendance_id}/employee_payslips</b></code> <code>(Admin only, Get all employee payslips)</code></summary>

##### Headers

> ```json
> {
>   "Authorization": "JWT {admin jwt.access.token}",
> }
> ```

##### Responses

> | http code | content-type               | response                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | reason                                       |
> | --------- | -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- |
> | `200`     | `application/json`         | `{'total_take_home': <Number>, 'payslips': [{'employee': {'id': <UUID>, 'username': <String>, 'base_salary': <Number>}, 'period': {'start_at': <Datetime>, 'end_at': <Datetime>}, 'attendance': {'total_days': <Number>, 'prorated_amount': <Number>}, 'overtimes': [{'date': <Datetime>, 'hours': <Number>, 'amount': <Number>}, ...], 'reimbursements': [{'description': <String>, 'amount': <Number>}, ...], 'summary': {'base_salary': <Number>, 'prorated_amount': <Number>, 'overtime_total': <Number>, 'reimbursement_total': <Number>, 'take_home_pay': <Number>}}, ...]}` | None                                         |
> | `400`     | `text/plain;charset=UTF-8` | `attendance is not processed`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Attendance period has not been processed     |
> | `401`     | `text/plain;charset=UTF-8` | `unauthorized`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Invalid/No provided `Authorization` header   |
> | `403`     | `text/plain;charset=UTF-8` | `authority error`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Provided `Authorization` header were invalid |
> | `403`     | `text/plain;charset=UTF-8` | `forbidden`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | You are not allowed to access this endpoint  |

</details>
