//! [`Redacted`] — a value that must not leak into logs, errors, or panics.

use core::fmt;
use core::str::FromStr;

/// A wrapper marking its contents as sensitive.
///
/// Its [`Debug`] and [`Display`] render `[redacted]` and never reveal the inner value, so a
/// secret — a database password, an API token, a private key — cannot escape through a log
/// line, an error message, a `panic!`, or a `#[derive(Debug)]` on a struct that happens to
/// contain it. This matters especially with [`crate::Partial`]: `build()` may log the resolved
/// configuration, and a layer routinely holds secrets, so wrapping them here keeps them out of
/// the log without the configuration crate having to guess which fields are sensitive.
///
/// The value is reachable only through [`Redacted::expose_secret`] or [`Redacted::into_inner`],
/// whose names make every deliberate exposure easy to audit (grep for `expose_secret`). There
/// is intentionally **no** `Deref`, `AsRef`, or `Serialize`: the value never escapes implicitly.
///
/// ```
/// use partial_config::Redacted;
///
/// let dsn = Redacted::new("postgres://user:hunter2@db/app".to_string());
/// assert_eq!(format!("{dsn:?}"), "[redacted]");
/// assert!(!format!("{dsn:?}").contains("hunter2"));
/// assert_eq!(dsn.expose_secret(), "postgres://user:hunter2@db/app");
/// ```
pub struct Redacted<T>(T);

impl<T> Redacted<T> {
    /// Wrap a value, marking it sensitive.
    pub const fn new(secret: T) -> Self {
        Self(secret)
    }

    /// Borrow the wrapped secret. Named so that every read is easy to find and audit.
    pub fn expose_secret(&self) -> &T {
        &self.0
    }

    /// Consume the wrapper and return the secret. The other auditable point of exposure.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T> fmt::Display for Redacted<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

impl<T> From<T> for Redacted<T> {
    fn from(secret: T) -> Self {
        Self(secret)
    }
}

/// Parse the inner value and wrap it — so a `Redacted<T>` is a drop-in configuration field,
/// sourced from an environment variable or a CLI flag exactly as the bare `T` would be.
impl<T: FromStr> FromStr for Redacted<T> {
    type Err = T::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        T::from_str(s).map(Self)
    }
}

impl<T: Clone> Clone for Redacted<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Used only as a discarded placeholder when [`crate::Partial::build`] reports a missing
/// required field; the value is never read.
impl<T: Default> Default for Redacted<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

/// Deserialize the inner value and wrap it, so a secret can be sourced from a config file.
/// There is deliberately no `Serialize`: a redacted value must not be written back out.
#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Redacted<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_and_display_never_reveal_the_secret() {
        let s = Redacted::new("postgres://user:hunter2@db/app".to_string());
        assert_eq!(format!("{s:?}"), "[redacted]");
        assert_eq!(format!("{s}"), "[redacted]");
        assert!(!format!("{s:?} {s}").contains("hunter2"));
    }

    #[test]
    fn the_value_is_reachable_only_through_the_named_accessors() {
        let s = Redacted::new(42_u16);
        assert_eq!(*s.expose_secret(), 42);
        assert_eq!(s.into_inner(), 42);
    }

    #[test]
    fn from_str_wraps_so_it_is_a_drop_in_config_field() {
        let port: Redacted<u16> = "8080".parse().unwrap();
        assert_eq!(*port.expose_secret(), 8080);

        // The inner parse error propagates unchanged.
        assert!("not-a-number".parse::<Redacted<u16>>().is_err());
    }

    /// The point of the type: a secret stays hidden even when it is a field of some other
    /// struct's derived `Debug`, while the non-secret fields around it print normally.
    #[test]
    fn a_secret_field_stays_hidden_inside_a_derived_debug() {
        #[derive(Debug)]
        #[allow(dead_code)] // read only through the derived `Debug`
        struct Config {
            dsn: Redacted<String>,
            port: u16,
        }

        let out = format!(
            "{:?}",
            Config {
                dsn: Redacted::new("top-secret-dsn".to_string()),
                port: 8080,
            }
        );
        assert!(out.contains("[redacted]"));
        assert!(!out.contains("top-secret-dsn"));
        assert!(out.contains("8080"), "non-secret fields still print");
    }
}
