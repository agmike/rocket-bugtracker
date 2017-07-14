use diesel::pg::PgConnection;
use r2d2::{Config, Pool, PooledConnection, GetTimeout};
use r2d2_diesel::ConnectionManager;
use rocket::request::{Request, Outcome, FromRequest};
use rocket::Outcome::{Success, Failure};
use rocket::http::Status;
use dotenv::dotenv;
use std::env;

fn create_db_pool() -> Pool<ConnectionManager<PgConnection>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let config = Config::default();
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::new(config, manager).expect("Failed to create pool.")
}

lazy_static! {
    pub static ref DB_POOL: Pool<ConnectionManager<PgConnection>> = create_db_pool();
}

pub struct DB(PooledConnection<ConnectionManager<PgConnection>>);

impl DB {
    pub fn conn(&self) -> &PgConnection {
        &*self.0
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for DB {
    type Error = GetTimeout;
    fn from_request(_: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        match DB_POOL.get() {
            Ok(conn) => Success(DB(conn)),
            Err(e) => Failure((Status::InternalServerError, e)),
        }
    }
}