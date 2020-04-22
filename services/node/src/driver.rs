use std::io::Error as IOError;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

pub struct DriverManager {
    path: String,
    driver: Mutex<Option<Child>>,
}

impl DriverManager {
    pub fn new(path: String) -> Self {
        DriverManager {
            path,
            driver: Mutex::new(None),
        }
    }

    pub fn start(&self) -> Result<(), IOError> {
        // Chrome needs some "special handling"
        let driver = if std::env::var("BROWSER").unwrap_or_default() == "chrome" {
            Command::new(self.path.clone())
                .arg("--whitelisted-ips")
                .arg("*")
                .stdout(Stdio::inherit())
                .spawn()
        } else {
            Command::new(self.path.clone())
                .stdout(Stdio::inherit())
                .spawn()
        };

        match driver {
            Ok(child) => {
                *self.driver.lock().unwrap() = Some(child);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub fn stop(&self) {
        let mut driver = self.driver.lock().unwrap();

        if let Some(d) = driver.as_mut() {
            d.kill().ok();
        };

        *driver = None;
    }
}
