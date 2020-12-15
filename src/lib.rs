use log::info;
use serde::Deserialize;

#[macro_use]
pub mod db;
pub mod date_time;
pub mod jwt;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
