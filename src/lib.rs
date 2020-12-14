use lazy_static::*;
use log::info;
use serde::*;

#[macro_use]
pub mod db;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
