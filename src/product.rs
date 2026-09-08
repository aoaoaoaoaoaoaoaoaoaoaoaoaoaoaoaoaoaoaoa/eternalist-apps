//! One product's closed identity.

use std::fmt;

/// A product's reverse-DNS identifier and display name.
///
/// The identifier is exactly three lowercase labels joined by `.`, each label
/// `[a-z0-9]` with interior `-`. It is the bundle identifier on macOS and
/// Windows, the source of the platform directory triple, and the crash-report
/// product. Declare it once in the product's contract crate and derive every
/// other spelling from it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductIdentity {
    identifier: &'static str,
    name: &'static str,
}

impl ProductIdentity {
    /// Declare a product. In a `const` context a malformed identifier or an
    /// empty name fails compilation.
    #[must_use]
    pub const fn declare(identifier: &'static str, name: &'static str) -> Self {
        assert!(
            lawful_identifier(identifier),
            "product identifier must be three lowercase reverse-DNS labels"
        );
        assert!(!name.is_empty(), "product name must not be empty");
        Self { identifier, name }
    }

    /// The reverse-DNS identifier.
    #[must_use]
    pub const fn identifier(self) -> &'static str {
        self.identifier
    }

    /// The display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// The last identifier label: the product's slug and platform directory name.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        self.label(2)
    }

    /// The `(qualifier, organization, application)` triple behind the identifier.
    #[must_use]
    pub const fn triple(self) -> (&'static str, &'static str, &'static str) {
        (self.label(0), self.label(1), self.label(2))
    }

    const fn label(self, index: usize) -> &'static str {
        let bytes = self.identifier.as_bytes();
        let mut start = 0;
        let mut seen = 0;
        let mut cursor = 0;
        while cursor < bytes.len() {
            if bytes[cursor] == b'.' {
                if seen == index {
                    break;
                }
                seen += 1;
                start = cursor + 1;
            }
            cursor += 1;
        }
        match std::str::from_utf8(bytes.split_at(cursor).0.split_at(start).1) {
            Ok(label) => label,
            Err(_) => unreachable!(),
        }
    }
}

impl fmt::Display for ProductIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.identifier)
    }
}

const fn lawful_identifier(identifier: &str) -> bool {
    let bytes = identifier.as_bytes();
    let mut labels = 0;
    let mut length = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        let boundary = cursor + 1 == bytes.len() || bytes[cursor + 1] == b'.';
        match byte {
            b'.' => {
                if length == 0 {
                    return false;
                }
                labels += 1;
                length = 0;
            }
            b'a'..=b'z' | b'0'..=b'9' => length += 1,
            b'-' if length > 0 && !boundary => length += 1,
            _ => return false,
        }
        cursor += 1;
    }
    labels == 2 && length > 0
}
