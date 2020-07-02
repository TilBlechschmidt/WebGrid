use std::{iter::Iterator, time::Duration};

pub struct Backoff {
    retries: u32,
    limit: u32,
    multiplier: u32,
    current: Duration,
}

impl Backoff {
    pub fn default() -> Self {
        Self {
            retries: 0,
            limit: 13,
            multiplier: 2,
            current: Duration::from_millis(25),
        }
    }
}

impl Iterator for Backoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        self.retries += 1;

        if self.retries > self.limit {
            None
        } else {
            self.current *= self.multiplier;
            Some(self.current)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_monotonically_increasing() {
        let mut backoff = Backoff::default();
        let mut previous = Duration::default();

        while let Some(duration) = backoff.next() {
            assert!(previous < duration);
            previous = duration;
        }
    }

    #[test]
    fn backoff_is_not_constant() {
        let mut backoff = Backoff::default();

        // Skip the first one as we can't calculate the delta properly
        let mut previous = backoff.next().unwrap();
        let mut previous_delta = Duration::default();

        while let Some(duration) = backoff.next() {
            let delta = duration - previous;
            assert!(previous_delta < delta);
            previous_delta = delta;
            previous = duration;
        }
    }
}
