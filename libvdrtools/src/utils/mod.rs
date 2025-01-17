pub use indy_utils::environment;

pub mod crypto;

#[macro_use]
pub mod logger;

#[allow(unused_macros)]
#[macro_use]
pub mod result;

#[cfg(test)]
pub use indy_utils::test;

#[macro_use]
pub mod try_utils;

pub use indy_api_types::validation;

pub use indy_utils::wql;

#[macro_use]
pub mod qualifier;

pub mod extensions;

macro_rules! map (
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);
