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

#[cfg(test)]
mod does {
    use super::*;

    #[test]
    fn consume_prefix() {
        let prefix = "hello ";
        let suffix = "world";
        let input = format!("{}{}", prefix, suffix);
        let mut string = MatchableString::new(&input);

        assert!(string.consume_prefix(prefix).is_some());
        assert_eq!(string.current(), suffix);
    }

    #[test]
    fn consume_prefix_repeatedly() {
        let prefix1 = "john ";
        let prefix2 = "eats ";
        let suffix = "potatoes";
        let input = format!("{}{}{}", prefix1, prefix2, suffix);
        let mut string = MatchableString::new(&input);

        assert!(string.consume_prefix(prefix1).is_some());
        assert!(string.consume_prefix(prefix2).is_some());
        assert_eq!(string.current(), suffix);
    }

    #[test]
    fn consume_count() {
        let input = "123456";
        let mut string = MatchableString::new(input);

        assert_eq!(string.consume_count(3), Some("123"));
        assert_eq!(string.current(), "456")
    }

    #[test]
    fn consume_count_repeatedly() {
        let input = "123456789";
        let mut string = MatchableString::new(input);

        assert_eq!(string.consume_count(3), Some("123"));
        assert_eq!(string.consume_count(4), Some("4567"));
        assert_eq!(string.current(), "89");
    }

    #[test]
    fn not_panic_on_empty_string() {
        let mut string = MatchableString::new("");
        assert!(string.current().is_empty());
        assert!(string.consume_count(3).is_none());
        assert!(string.consume_prefix("42").is_none());
    }
}
