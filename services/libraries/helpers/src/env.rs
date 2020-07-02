use log::error;
use serde::de::DeserializeOwned;
use serde_json::from_str;
use std::convert::From;
use std::env::var;

// Set by build.rs
const ENV_PREFIX: &str = env!("ENV_PREFIX", "Environment prefix not defined!");

pub fn get_env<T, D>(key: &str, default: Option<D>) -> T
where
    T: DeserializeOwned + From<D>,
{
    let key = format!("{}{}", ENV_PREFIX, key);
    let error_message = format!("Missing environment variable: {}", key);

    if let Ok(env_val) = var(key.clone()) {
        let raw_parse = from_str(&env_val);
        let string_parse = |_| from_str(&format!("\"{}\"", env_val));

        match raw_parse.or_else(string_parse) {
            Ok(val) => val,
            Err(e) => {
                error!(
                    "Unable to deserialize environment variable '{}': {}",
                    key,
                    e.to_string()
                );
                panic!(e);
            }
        }
    } else if let Some(default) = default {
        default.into()
    } else {
        error!("{}", error_message);
        panic!(error_message);
    }
}

// TODO Include doc-comments in macro output!
macro_rules! define_env {
    () => {};

    (@default $t:ty, $default:expr) => { Some($default) };
    (@default $t:ty) => { Option::<$t>::None };

    (
        $(
            $(#[$outer:meta])*
            ($name:ident, $t:ty, $key:literal$(, $default:expr)?)
        )*;
        $($rest:tt)*
    ) => {
        $(
            lazy_static::lazy_static! {
                pub static ref $name: $t = crate::env::get_env($key, define_env!(@default $t$(, $default)?));
            }
        )*
        define_env!($($rest)*);
    };

    ($namespace:ident => { $($inner:tt)* }; $($rest:tt)*) => {
        pub mod $namespace {
            define_env!($($inner)*);
        }
        define_env!($($rest)*);
    };
}

define_env! {
    global => {
        /// Base port number from which service ports are derived
        (BASE_PORT, u16, "BASE_PORT", 40001 as u16);
        /// Port for the liveness/readiness probe server
        /// (same for each service, needs to be enabled through ENV to prevent port collisions when used locally)
        (STATUS_PORT, u16, "STATUS_PORT", 47002 as u16);
        (STATUS_SERVER_ENABLED, bool, "STATUS_SERVER_ENABLED", false);
    };
    resources => {
        redis => {
            (URL, String, "REDIS_URL", "redis://webgrid-redis/");
        };
    };
    service => {
        manager => {
            (ID, String, "MANAGER_ID");
            (HOST, String, "MANAGER_HOST");
        };
        orchestrator => {
            (ID, String, "ORCHESTRATOR_ID");
            (SLOT_COUNT, usize, "SLOTS");
        };
        node => {
            (ID, String, "SESSION_ID");
            (DRIVER, String, "DRIVER");
            (DRIVER_PORT, u16, "DRIVER_PORT");
            (BROWSER, String, "BROWSER");
            // TODO Replace this with truly optional environment variables
            (ON_SESSION_CREATE, String, "ON_SESSION_CREATE", "/usr/bin/echo");
        };
    };
}
