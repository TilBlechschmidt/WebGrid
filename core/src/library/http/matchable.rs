/// Reader-like wrapper for string slices
///
/// This structure allows structured consumption of string slices by continually
/// matching expected parts of a string.
pub struct MatchableString<'a> {
    source: &'a str,
    index: usize,
}

impl<'a> MatchableString<'a> {
    /// Creates a new reader by wrapping a string reference
    #[inline]
    pub fn new(source: &'a str) -> Self {
        Self { source, index: 0 }
    }

    /// Returns the current unconsumed substring
    #[inline]
    pub fn current(&self) -> &'a str {
        &self.source[self.index..]
    }

    /// Consumes a fixed prefix if possible
    #[inline]
    pub fn consume_prefix(&mut self, prefix: &str) -> Option<()> {
        if self.current().starts_with(prefix) {
            self.index += prefix.len();
            Some(())
        } else {
            None
        }
    }

    /// Consumes and returns a number of characters
    #[inline]
    pub fn consume_count(&mut self, count: usize) -> Option<&'a str> {
        match self.current().get(..count) {
            Some(consumed) => {
                self.index += count;
                Some(consumed)
            }
            None => None,
        }
    }
}
