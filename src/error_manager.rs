use log::error;
use redis::{ErrorKind, RedisError};
use std::error::Error;

pub fn get_public_error(error: &RedisError) -> String {
    // Match kind of error
    error!("{:?} - {}", error.kind(), error.cause().unwrap());

    String::from("Couldn't connect to server, retry later")
}
