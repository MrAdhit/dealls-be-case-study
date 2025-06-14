from datetime import datetime, timezone
import string
from dotenv import load_dotenv
import docker
import psycopg2
import os
import subprocess
import hashlib
import socket
import time
import pytz
import requests
import pytest
import random

# Pretty hacky way of getting an available port
def get_available_port():
    sock = socket.socket()
    sock.bind(('', 0))
    port = sock.getsockname()[1]
    sock.close()
    
    return port

docker_client = docker.from_env()

docker_containers = []

def stop_containers(containers):
    for container in containers:
        container.stop()

def build_docker_image(dockerfile, image_name, label):
    try:
        # Get existing image and its creation timestamp
        img = docker_client.images.get(image_name)
        img_time = datetime.strptime(img.attrs['Created'].split('.')[0], '%Y-%m-%dT%H:%M:%S')
        img_time = img_time.replace(tzinfo=pytz.UTC)
        
        # Get dockerfile modification time
        file_time = datetime.fromtimestamp(os.path.getmtime(dockerfile), tz=timezone.utc)
        
        if file_time > img_time:
            print(f"Dockerfile newer than image, rebuilding {label} image")
            img, logs = docker_client.images.build(
                path=".",
                dockerfile=dockerfile,
                tag=image_name
            )
            for line in logs:
                print(f"[DOCKER LOG] {line}")
        else:
            print(f"Using existing {label} image")
            
    except:
        print(f"Building Docker {label} image")
        img, logs = docker_client.images.build(
            path=".",
            dockerfile=dockerfile,
            tag=image_name
        )
        for line in logs:
            print(f"[DOCKER LOG] {line}")
            
    return img

be_img = build_docker_image("be.dockerfile", "backend-test:latest", "Backend-Test")

def spin_environment(faketime: datetime, pg_vol: str):
    containers = []

    test_db_port = get_available_port()
    
    test_db_container = docker_client.containers.run(
        image="postgres:17",
        ports={ "5432/tcp": int(test_db_port) },
        environment={
          "POSTGRES_USER": "postgres",
          "POSTGRES_PASSWORD": "postgres",
        },
        volumes={
            f"{pg_vol}": { "bind": "/var/lib/postgresql/data", "mode": "rw" },
        },
        network="bridge",
        auto_remove=True,
        detach=True
    )
    containers.append(test_db_container)

    while True:
        time.sleep(1)
        container = docker_client.containers.get(test_db_container.id)
        
        if container.logs().decode().strip().endswith("database system is ready to accept connections"):
            time.sleep(2)
            break

        print("Waiting for test database to spin up")

    test_db_url = f"postgres://postgres:postgres@127.0.0.1:{test_db_port}/postgres"

    backend_port = get_available_port()
    backend_host = f"127.0.0.1:{backend_port}"

    migrator_process = subprocess.Popen(
        ["cargo", "run"],
        cwd="./migration",
        env=dict(os.environ, **{
            "DATABASE_URL": test_db_url,
        }),
    )
    migrator_process.wait()
    
    # Get host IP, because this approach is much much much more reliable
    gateway_ip = docker_client.networks.get("bridge").attrs["IPAM"]["Config"][0]["Gateway"]

    backend_container = docker_client.containers.run(
        image=be_img.id,
        ports={ f"{backend_port}/tcp": int(backend_port) },
        environment={
            "DATABASE_URL": f"postgres://postgres:postgres@{gateway_ip}:{test_db_port}/postgres",
            "HOST_ADDRESS": backend_host.replace("127.0.0.1", "0.0.0.0"),
            "JWT_SECRET": "secret",
            "FAKETIME": faketime.strftime('%Y-%m-%d %H:%M:%S'),
        },
        volumes={
            f"{pg_vol}": { "bind": "/var/lib/postgresql/data", "mode": "rw" },
        },
        network="bridge",
        auto_remove=True,
        detach=True
    )
    containers.append(backend_container)

    while True:
        container = docker_client.containers.get(backend_container.id)
        
        if "starting service:" in container.logs().decode().strip():
            break

        print("Waiting for test backend to spin up")
        time.sleep(1)
        
    print("A new environment is spun up:", (test_db_url, backend_host, faketime))
        
    return (test_db_url, f"http://{backend_host}", faketime, containers)

def generate_random_string(length, char_set=string.ascii_uppercase + string.digits):
    return ''.join(random.choice(char_set) for _ in range(length))

def create_user(pg_conn, faketime, username, password, role_type, salary):
    pg_curr = pg_conn.cursor()
    hashed_password = hashlib.sha256(f"{password}:{username}".encode()).digest()
    
    pg_curr.execute('INSERT INTO public."user" (username, created_at, updated_at, password, salary, role) VALUES (%s, %s, %s, %s, %s, %s::role_type);', (username, faketime, faketime, hashed_password, salary, role_type))
    pg_conn.commit()
    pg_curr.close()

def backend_login(backend_host, username, password):
    return requests.post(f"{backend_host}/auth/login", json={
        "username": username,
        "password": password
    })
    
def create_and_get_random_user(pg_conn, backend_host, faketime, role_type, salary):
    username, password = (generate_random_string(5), generate_random_string(5))

    create_user(
        pg_conn,
        faketime,
        username,
        password,
        role_type,
        salary
    )
    
    res = backend_login(backend_host, username, password)
    assert res.status_code == 200

    return res.text

### === The actual tests starts from here === ###
    
def test_auth(tmp_path):
    test_db_url, backend_host, faketime, containers = spin_environment(datetime(2024, 6, 2, 10, 0, 0), tmp_path)
    pg_conn = psycopg2.connect(test_db_url)

    username, password = ("Bob", "secret")
    create_user(
        pg_conn,
        faketime,
        username,
        password,
        "employee",
        5000000
    )
    
    res = backend_login(backend_host, username, password)
    assert res.status_code == 200

    token = res.text

    res = requests.get(f"{backend_host}/auth", headers={
        "Authorization": f"JWT {token}"
    })
    res_user = res.json()

    assert res_user["username"] == username
    
    stop_containers(containers)

def test_attendance(tmp_path):
    # Test on Monday 3rd, June 2024
    test_db_url, backend_host, faketime, containers = spin_environment(datetime(2024, 6, 3, 10, 0, 0), tmp_path)
    pg_conn = psycopg2.connect(test_db_url)

    employee = create_and_get_random_user(pg_conn, backend_host, faketime, "employee", 5000000)
    admin = create_and_get_random_user(pg_conn, backend_host, faketime, "admin", 0)

    res_forbidden = requests.post(f"{backend_host}/attendance", headers={
        "Authorization": f"JWT {employee}"
    }, json={
        "start_at": datetime(2024, 6, 1, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
        "end_at": datetime(2024, 6, 30, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
    })
    assert res_forbidden.status_code == 403

    res_bad_request = requests.post(f"{backend_host}/attendance", headers={
        "Authorization": f"JWT {admin}"
    }, json={
        "start_at": datetime(2024, 6, 30, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
        "end_at": datetime(2024, 6, 1, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
    })
    assert res_bad_request.status_code == 400
    assert res_bad_request.text == "end_at is lower than start_at"

    res_created = requests.post(f"{backend_host}/attendance", headers={
        "Authorization": f"JWT {admin}"
    }, json={
        "start_at": datetime(2024, 6, 1, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
        "end_at": datetime(2024, 6, 30, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
    })
    assert res_created.status_code == 201
    
    attendance_id = res_created.json()["id"]

    res_attended = requests.post(f"{backend_host}/attendance/{attendance_id}", headers={
        "Authorization": f"JWT {employee}"
    })
    assert res_attended.status_code == 201
    assert res_attended.json()["attendance_period_id"] == attendance_id

    res_attended_existed = requests.post(f"{backend_host}/attendance/{attendance_id}", headers={
        "Authorization": f"JWT {employee}"
    })
    assert res_attended_existed.status_code == 200
    assert res_attended_existed.json()["attendance_period_id"] == attendance_id
    
    stop_containers(containers)

def test_attendance_weekend(tmp_path):
    # Test on Sunday 2nd, June 2024
    test_db_url, backend_host, faketime, containers = spin_environment(datetime(2024, 6, 2, 10, 0, 0), tmp_path)
    pg_conn = psycopg2.connect(test_db_url)

    employee = create_and_get_random_user(pg_conn, backend_host, faketime, "employee", 5000000)
    admin = create_and_get_random_user(pg_conn, backend_host, faketime, "admin", 0)

    res_created = requests.post(f"{backend_host}/attendance", headers={
        "Authorization": f"JWT {admin}"
    }, json={
        "start_at": datetime(2024, 6, 1, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
        "end_at": datetime(2024, 6, 30, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
    })
    assert res_created.status_code == 201
    
    attendance_id = res_created.json()["id"]

    res_attended = requests.post(f"{backend_host}/attendance/{attendance_id}", headers={
        "Authorization": f"JWT {employee}"
    })
    assert res_attended.status_code == 400
    assert res_attended.text == "cannot attend on weekend"
    
    stop_containers(containers)

def test_employee_overtime(tmp_path):
    # Test on Monday 3rd, June 2024 5 PM (After work hour)
    test_db_url, backend_host, faketime, containers = spin_environment(datetime(2024, 6, 3, 18, 0, 0), tmp_path)
    pg_conn = psycopg2.connect(test_db_url)

    employee = create_and_get_random_user(pg_conn, backend_host, faketime, "employee", 5000000)
    admin = create_and_get_random_user(pg_conn, backend_host, faketime, "admin", 0)

    res_created = requests.post(f"{backend_host}/attendance", headers={
        "Authorization": f"JWT {admin}"
    }, json={
        "start_at": datetime(2024, 6, 1, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
        "end_at": datetime(2024, 6, 30, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
    })
    assert res_created.status_code == 201
    
    attendance_id = res_created.json()["id"]

    res_overtime_has_not_checked_in = requests.post(f"{backend_host}/attendance/{attendance_id}/overtime", headers={
        "Authorization": f"JWT {employee}"
    }, json={
        "extra_hours": 3
    })
    assert res_overtime_has_not_checked_in.status_code == 400
    assert res_overtime_has_not_checked_in.text == "you have not checked-in today"

    res_attended = requests.post(f"{backend_host}/attendance/{attendance_id}", headers={
        "Authorization": f"JWT {employee}"
    })
    # Still works because the requirement says that "No rules for late or early check-ins or check-outs; check-in at any time that day counts"
    assert res_attended.status_code == 201

    res_overtime_too_much_hours = requests.post(f"{backend_host}/attendance/{attendance_id}/overtime", headers={
        "Authorization": f"JWT {employee}"
    }, json={
        "extra_hours": 6
    })
    assert res_overtime_too_much_hours.status_code == 400
    assert res_overtime_too_much_hours.text == "you cannot take overtime for more than 3 hours a day"

    res_overtime = requests.post(f"{backend_host}/attendance/{attendance_id}/overtime", headers={
        "Authorization": f"JWT {employee}"
    }, json={
        "extra_hours": 1
    })
    assert res_overtime.status_code == 201
    assert res_overtime.json()["attendance_period_id"] == attendance_id
    
    res_overtime_update = requests.post(f"{backend_host}/attendance/{attendance_id}/overtime", headers={
        "Authorization": f"JWT {employee}"
    }, json={
        "extra_hours": 2
    })
    assert res_overtime_update.status_code == 200 # Because it updates existing overtime entry
    assert res_overtime_update.json()["id"] == res_overtime.json()["id"]

    res_overtime_update_too_much_hours = requests.post(f"{backend_host}/attendance/{attendance_id}/overtime", headers={
        "Authorization": f"JWT {employee}"
    }, json={
        "extra_hours": 1
    })
    assert res_overtime_update_too_much_hours.status_code == 400
    assert res_overtime_update_too_much_hours.text == "you cannot take overtime for more than 3 hours a day"
    
    stop_containers(containers)

def test_employee_reimburse(tmp_path):
    # Test on Monday 3rd, June 2024
    test_db_url, backend_host, faketime, containers = spin_environment(datetime(2024, 6, 3, 10, 0, 0), tmp_path)
    pg_conn = psycopg2.connect(test_db_url)

    employee = create_and_get_random_user(pg_conn, backend_host, faketime, "employee", 5000000)
    admin = create_and_get_random_user(pg_conn, backend_host, faketime, "admin", 0)

    res_created = requests.post(f"{backend_host}/attendance", headers={
        "Authorization": f"JWT {admin}"
    }, json={
        "start_at": datetime(2024, 6, 1, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
        "end_at": datetime(2024, 6, 30, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
    })
    assert res_created.status_code == 201
    
    attendance_id = res_created.json()["id"]

    res_reimburse = requests.post(f"{backend_host}/attendance/{attendance_id}/reimburse", headers={
        "Authorization": f"JWT {employee}"
    }, json={
        "description": "Commute",
        "amount": 20_000
    })
    assert res_reimburse.status_code == 201
    assert res_reimburse.json()["description"] == "Commute"
    assert res_reimburse.json()["amount"] == 20_000
    assert res_reimburse.json()["attendance_period_id"] == attendance_id
    
    stop_containers(containers)

def test_payroll(tmp_path):
    def first_day():
        # Test on Monday 3rd, June 2024
        test_db_url, backend_host, faketime, containers = spin_environment(datetime(2024, 6, 3, 10, 0, 0), tmp_path)
        pg_conn = psycopg2.connect(test_db_url)

        employee_1 = create_and_get_random_user(pg_conn, backend_host, faketime, "employee", 5000000)
        employee_2 = create_and_get_random_user(pg_conn, backend_host, faketime, "employee", 5000000)

        admin = create_and_get_random_user(pg_conn, backend_host, faketime, "admin", 0)

        res_created = requests.post(f"{backend_host}/attendance", headers={
            "Authorization": f"JWT {admin}"
        }, json={
            "start_at": datetime(2024, 6, 3, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
            "end_at": datetime(2024, 6, 5, 0, 0).replace(tzinfo=pytz.UTC).isoformat(),
        })
        assert res_created.status_code == 201
        
        attendance_id = res_created.json()["id"]

        res_attended_e_1 = requests.post(f"{backend_host}/attendance/{attendance_id}", headers={
            "Authorization": f"JWT {employee_1}"
        })
        assert res_attended_e_1.status_code == 201

        res_attended_e_2 = requests.post(f"{backend_host}/attendance/{attendance_id}", headers={
            "Authorization": f"JWT {employee_2}"
        })
        assert res_attended_e_2.status_code == 201

        res_reimburse_e_1 = requests.post(f"{backend_host}/attendance/{attendance_id}/reimburse", headers={
            "Authorization": f"JWT {employee_1}"
        }, json={
            "description": "Commute",
            "amount": 20_000
        })
        assert res_reimburse_e_1.status_code == 201
        assert res_reimburse_e_1.json()["description"] == "Commute"
        assert res_reimburse_e_1.json()["amount"] == 20_000
        assert res_reimburse_e_1.json()["attendance_period_id"] == attendance_id

        res_reimburse_e_2 = requests.post(f"{backend_host}/attendance/{attendance_id}/reimburse", headers={
            "Authorization": f"JWT {employee_2}"
        }, json={
            "description": "Office supplies",
            "amount": 100_000
        })
        assert res_reimburse_e_2.status_code == 201
        assert res_reimburse_e_2.json()["description"] == "Office supplies"
        assert res_reimburse_e_2.json()["amount"] == 100_000
        assert res_reimburse_e_2.json()["attendance_period_id"] == attendance_id

        stop_containers(containers)
        
        return (employee_1, employee_2, admin, attendance_id)

    employee_1, employee_2, admin, attendance_id = first_day()

    def second_day():
        # Test on Tuesday 4th, June 2024 6 PM
        _, backend_host, _, containers = spin_environment(datetime(2024, 6, 4, 18, 0, 0), tmp_path)

        res_attended_e_1 = requests.post(f"{backend_host}/attendance/{attendance_id}", headers={
            "Authorization": f"JWT {employee_1}"
        })
        assert res_attended_e_1.status_code == 201
        
        # Employee 2 is not attending the second day

        res_reimburse_e_1 = requests.post(f"{backend_host}/attendance/{attendance_id}/reimburse", headers={
            "Authorization": f"JWT {employee_1}"
        }, json={
            "description": "Commute",
            "amount": 20_000
        })
        assert res_reimburse_e_1.status_code == 201
        assert res_reimburse_e_1.json()["description"] == "Commute"
        assert res_reimburse_e_1.json()["amount"] == 20_000
        assert res_reimburse_e_1.json()["attendance_period_id"] == attendance_id
        
        # So employee 1 took some overtime

        res_overtime = requests.post(f"{backend_host}/attendance/{attendance_id}/overtime", headers={
            "Authorization": f"JWT {employee_1}"
        }, json={
            "extra_hours": 3
        })
        assert res_overtime.status_code == 201
        assert res_overtime.json()["attendance_period_id"] == attendance_id
        
        return (backend_host, containers)
        
    backend_host, containers = second_day()

    res_process_payroll = requests.post(f"{backend_host}/attendance/{attendance_id}/process_payroll", headers={
        "Authorization": f"JWT {admin}"
    })
    assert res_process_payroll.status_code == 200
    assert res_process_payroll.json()["processed"] == True
    
    # Expected calculations for employee 1
    #
    # Working days from period (2024-06-03 to 2024-06-05) = 3 days
    # Attendance days = 2 days
    # Base hourly rate = salary / (working days * working hours) = 5mil / (3 days * 8 hours) = 208,333.33 / hour
    # Overtime rate = hourly rate * 2 * overtime hours = 208,333.33 * 2 * 3 hours = 1,249,998
    # Prorated base = salary * attendance days / working days = 5mil * 2 days / 3 days = 3,333,333
    # Reimbursement = 20k * 2 (Commute) = 40k
    # Total = prorated base + overtime bonus + reimbursement = 3,333,333 + 1,249,998 + 40k = 4,623,331

    res_payslip_e_1 = requests.get(f"{backend_host}/attendance/{attendance_id}/payslip", headers={
        "Authorization": f"JWT {employee_1}"
    })
    assert res_payslip_e_1.status_code == 200
    assert res_payslip_e_1.json()["attendance"]["total_days"] == 2
    assert res_payslip_e_1.json()["summary"]["base_salary"] == 5000000
    assert res_payslip_e_1.json()["summary"]["prorated_amount"] == 3333333
    assert res_payslip_e_1.json()["summary"]["overtime_total"] == 1249998
    assert res_payslip_e_1.json()["summary"]["reimbursement_total"] == 40000
    assert res_payslip_e_1.json()["summary"]["take_home_pay"] == 4623331

    # Expected calculations for employee 2
    #
    # Working days from period (2024-06-03 to 2024-06-05) = 3 days
    # Attendance days = 1 days
    # Base hourly rate = salary / (working days * working hours) = 5mil / (3 days * 8 hours) = 208,333.33 / hour
    # Overtime rate = hourly rate * 2 * overtime hours = 208,333.33 * 2 * 0 hours = 0
    # Prorated base = salary * attendance days / working days = 5mil * 1 days / 3 days = 1,666,666
    # Reimbursement = 20k * 1 (Office supplies) = 100k
    # Total = prorated base + overtime bonus + reimbursement = 1,666,666 + 0 + 100k = 1,766,666

    res_payslip_e_2 = requests.get(f"{backend_host}/attendance/{attendance_id}/payslip", headers={
        "Authorization": f"JWT {employee_2}"
    })
    assert res_payslip_e_2.status_code == 200
    assert res_payslip_e_2.json()["attendance"]["total_days"] == 1
    assert res_payslip_e_2.json()["summary"]["base_salary"] == 5000000
    assert res_payslip_e_2.json()["summary"]["prorated_amount"] == 1666666
    assert res_payslip_e_2.json()["summary"]["overtime_total"] == 0
    assert res_payslip_e_2.json()["summary"]["reimbursement_total"] == 100000
    assert res_payslip_e_2.json()["summary"]["take_home_pay"] == 1766666
    
    res_employee_payslips = requests.get(f"{backend_host}/attendance/{attendance_id}/employee_payslips", headers={
        "Authorization": f"JWT {admin}"
    })
    assert res_employee_payslips.status_code == 200
    assert res_employee_payslips.json()["total_take_home"] == res_payslip_e_1.json()["summary"]["take_home_pay"] + res_payslip_e_2.json()["summary"]["take_home_pay"]

    stop_containers(containers)
