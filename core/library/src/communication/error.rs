use crate::BoxedError;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// Type erased, serializable error which retains the error chain information
///
/// This error is used to retain information about errors and their causes when
/// sending an `Err(_)` over the wire. While receiving services don't know about
/// the possible types of errors, they can still use this error type to embed in
/// their own Errors and display meaningful information to API consumers.
///
/// When the Error from which this is created contains another BlackboxError in its
/// source chain, it will be consumed and integrated so that one nicely formatted
/// stacktrace can be provided at the top-most level.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct BlackboxError(Vec<String>);

impl BlackboxError {
    /// Creates a new instance from any error type
    ///
    /// Due to std providing default implementations for the `From<T> where T: T` trait,
    /// we can't both implement `Error` and `From<Error>` and have to decide on one.
    pub fn new<E: Error + 'static>(e: E) -> Self {
        (&e as &(dyn Error + 'static)).into()
    }

    /// Creates a new instance from a boxed error type
    pub fn from_boxed(e: BoxedError) -> Self {
        (e.as_ref() as &(dyn Error + 'static)).into()
    }

    /// Consumes the error and returns the underlying list of causes
    pub fn into_causes(self) -> Vec<String> {
        self.0
    }
}

#[cfg(test)]
impl BlackboxError {
    fn new_with_causes(causes: Vec<String>) -> Self {
        Self(causes)
    }
}

impl Error for BlackboxError {}

impl Display for BlackboxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(first) = self.0.first() {
            writeln!(f, "Error: {}", first)?;
        } else {
            writeln!(f, "Unknown error")?;
            return Ok(());
        }

        writeln!(f, "\nCaused by:")?;
        for (index, cause) in self.0.iter().skip(1).enumerate() {
            writeln!(f, "    {}: {}", index, cause)?;
        }

        Ok(())
    }
}

impl From<&(dyn Error + 'static)> for BlackboxError {
    fn from(e: &(dyn Error + 'static)) -> Self {
        let mut source: Option<&(dyn Error + 'static)> = Some(e);
        let mut causes: Vec<String> = Vec::new();

        while let Some(error) = source {
            // Integrate any child BlackboxErrors and use ToString for anything else
            if let Some(blackbox_error) = error.downcast_ref::<BlackboxError>() {
                let mut child_causes = blackbox_error.0.clone();
                causes.append(&mut child_causes);
            } else {
                causes.push(error.to_string());
            }

            source = error.source();
        }

        Self(causes)
    }
}

mod does {
    use super::*;
    use thiserror::Error;

    #[derive(Error, Debug)]
    enum TestError {
        #[error("Internal error")]
        Internal(#[from] BlackboxError),
    }

    #[test]
    fn handle_no_cause() {
        let error = BlackboxError::new_with_causes(Vec::new());
        assert_eq!(error.to_string(), "Unknown error\n");
    }

    #[test]
    fn consume_nested() {
        let lower_error =
            BlackboxError::new_with_causes(vec![String::from("cause1"), String::from("cause2")]);
        let middle_error = TestError::from(lower_error);
        let high_error = BlackboxError::from(&middle_error as &(dyn Error + 'static));

        assert_eq!(high_error.0, vec!["Internal error", "cause1", "cause2"])
    }

    #[test]
    fn format_correctly() {
        let formatted = BlackboxError::new_with_causes(vec![
            String::from("cause1"),
            String::from("cause2"),
            String::from("cause3"),
        ])
        .to_string();

        assert_eq!(
            formatted,
            r#"Error: cause1

Caused by:
    0: cause2
    1: cause3
"#
        )
    }
}
