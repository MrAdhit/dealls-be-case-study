use std::{env, net::{SocketAddr, ToSocketAddrs as _}};

use sea_orm::ConnectOptions;
use tracing::info;

pub struct Config {
    pub host_address: SocketAddr,

    pub database_opt: ConnectOptions,
    
    pub jwt_key: String,
}

pub fn load() -> Config {
    Config {
        host_address: load_host_address(),
        database_opt: load_database_opt().into(),
        jwt_key: load_jwt_key(),
    }
}

fn load_host_address() -> SocketAddr {
    info!("Loading environment `HOST_ADDRESS`");

    let var = env::var("HOST_ADDRESS").unwrap_or_else(|_| "127.0.0.1:0".to_string());
    
    var.to_socket_addrs()
        .expect("`HOST_ADDRESS` is not in a valid format").nth(0)
        .expect("unable to resolve host from `HOST_ADDRESS`")
}

fn load_database_opt() -> impl Into<ConnectOptions> {
    info!("Loading environment `DATABASE_URL`");
    
    let var = env::var("DATABASE_URL").expect("Environment `DATABASE_URL` is required to be set");
    
    var
}

fn load_jwt_key() -> String {
    info!("Loading environment `JWT_SECRET`");

    let var = env::var("JWT_SECRET").expect("Environment `JWT_SECRET` is required to be set");
    
    var
}
