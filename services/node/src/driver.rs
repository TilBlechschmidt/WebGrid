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
        let driver = Command::new(self.path.clone())
            .arg("--port")
            .arg("3031")
            .stdout(Stdio::piped())
            .spawn();

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
