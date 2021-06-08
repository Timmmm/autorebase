use std::ops::Deref;

/// Trait to allow trimming ascii whitespace from a &[u8].
pub trait TrimAsciiWhitespace {
    /// Trim ascii whitespace (based on `is_ascii_whitespace()`) from the
    /// start and end of a slice.
    fn trim_ascii_whitespace(&self) -> &[u8];
}

impl<T: Deref<Target = [u8]>> TrimAsciiWhitespace for T {
    fn trim_ascii_whitespace(&self) -> &[u8] {
        let from = match self.iter().position(|x| !x.is_ascii_whitespace()) {
            Some(i) => i,
            None => return &self[0..0],
        };
        let to = self.iter().rposition(|x| !x.is_ascii_whitespace()).unwrap();
        &self[from..=to]
    }
}

#[cfg(test)]
mod test {
    use super::TrimAsciiWhitespace;

    #[test]
    fn basic_trimming() {
        assert_eq!(" A ".as_bytes().trim_ascii_whitespace(), "A".as_bytes());
        assert_eq!(" AB ".as_bytes().trim_ascii_whitespace(), "AB".as_bytes());
        assert_eq!("A ".as_bytes().trim_ascii_whitespace(), "A".as_bytes());
        assert_eq!("AB ".as_bytes().trim_ascii_whitespace(), "AB".as_bytes());
        assert_eq!(" A".as_bytes().trim_ascii_whitespace(), "A".as_bytes());
        assert_eq!(" AB".as_bytes().trim_ascii_whitespace(), "AB".as_bytes());
        assert_eq!(" A B ".as_bytes().trim_ascii_whitespace(), "A B".as_bytes());
        assert_eq!("A B ".as_bytes().trim_ascii_whitespace(), "A B".as_bytes());
        assert_eq!(" A B".as_bytes().trim_ascii_whitespace(), "A B".as_bytes());
        assert_eq!(" ".as_bytes().trim_ascii_whitespace(), "".as_bytes());
        assert_eq!("  ".as_bytes().trim_ascii_whitespace(), "".as_bytes());
    }
}
