use lazy_static::*;
use log::info;
use serde::{Deserialize, Deserializer};

#[macro_use]
pub mod db;
pub mod jwt;
pub mod date_time;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
