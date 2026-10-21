// note: currently, this object will be shared between threads via an Arc
// however in future tihs should be changed to a request/response channel system for performance

use std::mem;

use bcrypt::DEFAULT_COST;
use rusqlite::{
    Connection,
    Error::{QueryReturnedMoreThanOneRow, QueryReturnedNoRows},
};

use crate::db::DbError::{IncorrectCredentials, Sqlite};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("auth token is either missing or invalid")]
    NotAuthorized,
    #[error("incorrect credentials")]
    IncorrectCredentials,
    #[error("account already exists")]
    AccountAlreadyExists,
    #[error("account doesn't exist")]
    AccountDoesntExist,
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("bcrypt error: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
}

pub struct DatabaseController {
    conn: Connection,
}

impl DatabaseController {
    pub fn new(address: &str) -> Result<Self, DbError> {
        let conn = Connection::open(address)?;
        Ok(Self { conn })
    }

    /// Perform the database migrations to set up the database.
    /// Will not perform any migrations if the tables already exist.
    /// Returns true if migrations were applied, and false if they are not required.
    pub fn perform_migrations(&self) -> Result<bool, DbError> {
        println!("performing DB migrations");
        if self
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='users'")?
            .exists(())?
        {
            println!("DB migrations not required");
            return Ok(false);
        }
        self.conn.execute_batch(include_str!("migrations/up.sql"))?;
        Ok(true)
    }

    /// Invalidates an existing access token
    pub fn logout_user(&mut self, uid: u32, token: [u8; 32]) -> Result<(), DbError> {
        if !self.verify_token(token, uid)? {
            return Err(DbError::NotAuthorized);
        }

        self.conn.execute(
            "DELETE FROM tokens WHERE token = ?1 AND uid = ?2",
            (token, uid),
        )?;
        Ok(())
    }

    /// Logs in a user, returning the access token.
    pub fn login_user(
        &mut self,
        username: String,
        password: String,
    ) -> Result<(u32, [u8; 32]), DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT uid, password_bcrypt FROM users WHERE username = ?1")?;

        let result = stmt.query_row([&username], |row| {
            Ok((row.get(0)?, row.get::<_, String>(1)?))
        });
        match result {
            Ok((uid, password_hash)) => match bcrypt::verify(password, &password_hash) {
                Ok(true) => {
                    mem::drop(stmt);
                    Ok((uid, self.generate_token(uid)?))
                }
                Ok(false) => Err(IncorrectCredentials),
                Err(why) => Err(DbError::Bcrypt(why)),
            },
            Err(QueryReturnedNoRows) => Err(DbError::IncorrectCredentials),
            Err(why) => Err(DbError::Sqlite(why)),
        }
    }

    /// Registers a new user, returning the user ID, and access token.
    /// If the user with that username already exists, an error will be returned
    pub fn register_user(
        &mut self,
        username: String,
        password: String,
    ) -> Result<(u32, [u8; 32]), DbError> {
        // FIXME input validation should happen here, not on the client!

        // check if the user exists already
        let mut stmt = self
            .conn
            .prepare("SELECT uid FROM users WHERE username = ?1")?;
        match stmt.query_one([&username], |_| Ok(())) {
            Ok(_) => return Err(DbError::AccountAlreadyExists),
            Err(QueryReturnedNoRows) => {}
            Err(why) => return Err(DbError::Sqlite(why)),
        }

        // hash the password
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;

        // create the user
        let uid: u32 = self.conn.query_row(
            "INSERT INTO users (
                username,
                password_bcrypt
            ) VALUES (?1, ?2)
            RETURNING uid",
            (username, hash),
            |row| row.get(0),
        )?;

        // create an access token
        mem::drop(stmt);
        let token = self.generate_token(uid)?;

        Ok((uid, token))
    }

    fn generate_token(&mut self, uid: u32) -> Result<[u8; 32], DbError> {
        let token = rand::random::<[u8; 32]>();
        self.conn.execute(
            "INSERT INTO tokens (
                token,
                uid
            ) VALUES (?1, ?2)",
            (token, uid),
        )?;
        return Ok(token);
    }

    // TODO: cache these for faster responses
    /// Checks if an authentication token is valid for a given user ID.
    fn verify_token(&mut self, token: [u8; 32], user: u32) -> Result<bool, DbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT uid FROM tokens WHERE token = ?1")?;
        match stmt.query_row([token], |row| row.get(0)) {
            Ok(uid) => {
                if user == uid {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(Sqlite(e)),
        }
    }
}
