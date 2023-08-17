use itertools::{Itertools, Position};

/// Simple *-based glob matching.
pub fn glob_match(pattern: &str, string: &str) -> bool {
    // Special handling is needed for this case.
    if pattern.is_empty() {
        return string.is_empty();
    }

    // Split by *.
    let parts = pattern.split('*');

    // Current index in string.
    let mut index = 0;

    for (position, part) in parts.with_position() {
        match position {
            Position::First => {
                // If it's the very first one then it must match at the beginning,
                // otherwise there was a * before so we take the first match.
                if !string.starts_with(part) {
                    return false;
                }
                index += part.len();
            }
            Position::Middle => {
                // In the middle there must be a * before, so we find the
                // first match.
                match string[index..].find(part) {
                    Some(offset) => {
                        index += offset + part.len();
                    }
                    None => {
                        return false;
                    }
                }
            }
            Position::Last => {
                // If this is the last part then the remaining string must end
                // with this.
                if !string[index..].ends_with(part) {
                    return false;
                }
            }
            Position::Only => {
                return pattern == part;
            }
        }
    }
    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn glob_test() {
        assert!(glob_match("", ""));
        assert!(glob_match("*", ""));
        assert!(glob_match("*", "a"));
        assert!(glob_match("*", "ab"));
        assert!(glob_match("*a", "a"));
        assert!(glob_match("a*", "a"));
        assert!(glob_match("a*a", "aa"));
        assert!(glob_match("a*a", "aba"));

        assert!(glob_match("**", ""));
        assert!(glob_match("**", "a"));
        assert!(glob_match("**", "ab"));
        assert!(glob_match("**a", "a"));
        assert!(glob_match("a**", "a"));
        assert!(glob_match("a**a", "aa"));
        assert!(glob_match("a**a", "aba"));

        assert!(glob_match("*a*a*", "aba"));

        assert!(!glob_match("", "a"));
        assert!(!glob_match("*a", "b"));
        assert!(!glob_match("a*", "b"));
        assert!(!glob_match("*a", "ab"));
        assert!(!glob_match("a*", "ba"));

        assert!(glob_match("a*bcd*bcd*ef", "aabcdbcdbcdabcdefefefggef"));
        assert!(!glob_match("a*bcd*bcd*ef", "abcdef"));
    }
}
