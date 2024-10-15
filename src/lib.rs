//! Layered partial configuration for Rust command-line applications. 
//!
//! Think back to the last time you wrote a command line application that had to be deployed via
//! Docker or some other containerisation service. You need to be able to specify the configuration
//! via the Environment variables, the command line flags, and some configuration file. The basic
//! idea is that if you specify something in the text file, then you want to be able to override it
//! with the environment variables for experimentation.
//!
//! We can take this further and consider what might happen if we wanted to override some `export
//! CONFIGURATION_VARIABLE=something` directive, with the CLI interface, and utilise the fact that
//! the excellent [`clap`](https://docs.rs/clap/latest/clap/) package provides an intuitive way to
//! navigate configuration options.
//!
//! So this pacakge provides you with two traits: `HasPartial` and `Source` which allow you to do 
//! two things: 
//!   - `HasPartial` allows you to declare a structure that you will use as a configuration as having
//!     a sister structure that consists of properly handled `Option` values for both optional values
//!     that might not have to be specified in the same layer, and optional values, not specifying
//!     which can reasonably be replaced with a fallback. With the `derive` feature enabled, you are
//!     able to automatically implement this feature and generate a new structure with the specified
//!     name.
//!
//!   - `Source` which is used to specify that some object in some way can be used to obtain a layer
//!     of configuraiton. For example, with the `serde` and `toml` features enabled there is an
//!     implementation of `Source` for the `path`-like objects that automatically resolves to a
//!     configuration layer for any structure for which `serde::Deserialize` can be automatically
//!     derived. We plan to add a custom approach to this, because they way that `serde` handles error
//!     reporting is not suitable for configuration files potentially written by humans. 
//!
//! # Examples
//!
//! ## HasPartial
//!
//! ```rust
//! #[derive(partial_config::HasPartial)]
//! #[partial_derives(Clone)]
//! #[partial_rename(CustomNameForPartialConfigurationStruct)]
//! pub struct Configuration {
//!     file_name: String, 
//!     port: u16, 
//!
//!     configuration_file: Option<String>,
//! }
//! ```
//!
//! This example demonstrates the simplest usage of the derive macro. All it does, is it generates
//! a new structure which is by default just `Partial<NameOfStructType>` but can be changed to
//! whatever you like via the `partial_rename` attribute as seen above. The generated structure can
//! derive some traits, which is helpful in case writing a manual `impl` block is challenging. 
//!
//! The macro recognises the `Option<String>` as an optional argument, meaning that if it's not
//! present in any layer, it's just set to `None`. It does recognise that `port` is not optional,
//! and if it is not specified in any layer, when the [`Partial::build`] function is invoked, an
//! error is reported. 
//!
//! As a common courtesy to the users, it's considered a good idea to report all missing or
//! malformed configuration options at once, instead of waiting for the user to fix the first
//! problem and then report the next one. In order to do so, you as the programmer would have to
//! write a little bit of tedious code, which you get for free by simply deriving [`HasPartial`] on
//! your type. 

use core::fmt::Debug;
mod error;

pub use error::{Error, MissingField};

#[cfg(feature = "derive")]
pub use partial_config_derive::HasPartial;

#[cfg(feature = "derive")]
pub use partial_config_derive::EnvSourced;

/// Implementors of this trait are considered partial states of the full configuration structure
/// which is [`Partial::Target`] in this case. If you are implementing this trait manually, pay
/// close attention to the documentation of the provided methods. If your partial structure
/// contains `Option`s only and is 1/1 correspondent to [`Partial::Target`] I would recommend
/// either using the [`partial_config::HasPartial`] derive macro, or if you want to avoid using
/// `syn`, to just `cargo expand` on the generated code and inline it. 
pub trait Partial: Default {
    /// The full configuration for which this type is considered a partial state obtained from a
    /// configuration layer.
    type Target: HasPartial<Partial = Self>;

    /// Error type returned from [`Partial::build`], [`Partial::source`] and
    /// [`Partial::override_with`]. If in doubt, just use [`partial_config::error::Error`].
    type Error: Debug;

    /// If at this point, all of the layers have been appropriately collected using the
    /// [`Partial::override_with`], we have all of the information that we can obtain. We try to
    /// construct [`Partial::Target`] with the information that we have obtained from other
    /// sources, and report any missing fields. Keep in mind that the correct implementation
    /// **should** at least attempt to report all missing or malformed fields at once, instead of
    /// failing as soon as the first one is identified. 
    fn build(self) -> Result<Self::Target, Self::Error>;

    /// Obtain [`Self`] from an object that is known to be a [`Source`] of the appropriate partial
    /// configuraiton. You should not override this function, unless you want to change the
    /// reporting. 
    ///
    /// One thing to keep in mind is that the [`Partial::source`] function calls produce the
    /// overriding pattern in the exact order in which they are applied. So 
    /// ```rust
    /// PartialConfiguration::default().source(file).source(EnvVars).source(CliArgs)
    /// ```
    /// shall read the file first, override any file specified in the file with the value specified
    /// in the environment variables and override any of those with the CLI arguments and not the
    /// reverse order. 
    fn source<T: Source<Self::Target>>(self, value: T) -> Result<Self, Self::Error>
    where
        <Self as Partial>::Error: From<<T as Source<<Self as Partial>::Target>>::Error>,
    {
        #[cfg(feature = "tracing")]
        tracing::info!("Sourcing configuration from `{}`", value.name());
        #[cfg(feature = "log")]
        log::info!("Sourcing configuration from `{}`", value.name());
        #[cfg(not(any(feature = "tracing", feature = "log")))]
        println!("Sourcing configuration from `{}`", value.name());
        let partial = value.to_partial()?;
        Ok(self.override_with(partial))
    }

    /// If `other` contains values that are specified and different from `self`, or `self` is
    /// empty, replace the value with the other. Otherwise keep the one that is specified, so if
    /// `self` has a value specified, and `other` has `None`, keep the `Some` value.
    fn override_with(self, other: Self) -> Self;
}

/// Marker trait that is used to allow a `derive` macro to generate a new structure. This trait is
/// useful for doign some trait-level contraining, but otherwise has no useful data. 
pub trait HasPartial {
    /// The type that represents a partial state of `Self` obtained from a single configuration
    /// source. The idea is that you can build up many instances of [`HasPartial::Partial`],
    /// combine them with [`Partial::override_with`] and then use them to build a single instance
    /// of `Self`. 
    type Partial: Partial<Target = Self>;
}

/// The implementor of this trait is a source of configuration. The method [`Source::to_partial`]
/// obtains a single layer of configuration and from a given source. 
///
/// This trait is mostly used for trait-level type checking so that the [`Partial::source`] method
/// operates as expected. No user is ever expected to call [`Source::to_partial`] directly.
pub trait Source<C: HasPartial> {
    type Error: Debug;

    /// Obtain a partial layer from `Self`. Not user facing, but used inside the
    /// [`Partial::source`] for type checking.
    fn to_partial(self) -> Result<C::Partial, Self::Error>;

    /// The name that is being printed whenever this layer of configuration is being parsed. If you
    /// came across this method to silence the `Sourcing configuration from XXX` message, instead
    /// simply override the [`Partial::source`] method instead. 
    fn name(&self) -> String;
}

impl<T, C, E> Source<C> for Option<T>
where
    C: HasPartial,
    T: Source<C, Error = E>,
    E: Debug,
{
    type Error = E;

    fn to_partial(self) -> Result<C::Partial, E> {
        self.map_or_else(|| Ok(C::Partial::default()), |v| v.to_partial())
    }

    fn name(&self) -> String {
        self.as_ref().map_or("Unspecified".to_owned(), |v| v.name())
    }
}

pub mod env {
    /// This is a marker trait that signals that this particular
    /// partial configuration has an environment variables source that
    /// is generated by the procedural macros. It doesn't do anything
    /// by itself, you need to derive the trait to create a new struct
    /// that will do env var sourcing in a reasonable way.
    pub trait EnvSourced<'a>: super::HasPartial + Sized {
        type Source: 'a + super::Source<Self> + Default;
    }

    /// Extract a string that corresponds to a consistent
    /// specification from an environment variable
    ///
    /// # Errors
    ///
    ///     - If any specified candidate environment variables has two
    ///     different specifications
    ///
    /// # Warns
    ///
    /// These are some conditions that are reported, but don't result
    /// in an `Err` variant being constructed.
    ///
    ///     - `None` is returned if neither of the candidate environment
    ///     variables was present, or all contained non-unicode values.
    ///
    ///     - If two candidates are set to the same value, a warning is
    ///     printed.
    ///
    ///     - If either one of the candidates is set to a non-unicode
    ///     value, a warning is printed.
    pub fn extract(candidates: &[&str]) -> Result<Option<String>, super::Error> {
        let mut found = None;
        for candidate in candidates {
            match (&found, std::env::var(candidate)) {
                (_, Err(std::env::VarError::NotPresent)) => continue,
                (_, Err(std::env::VarError::NotUnicode(thing))) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("The value of the environment variable for `{candidate}` was not Unicode. Got {thing:?}");
                    #[cfg(feature = "log")]
                    log::warn!("The value of the environment variable for `{candidate}` was not Unicode. Got {thing:?}");
                    #[cfg(not(any(feature = "log", feature = "tracing")))]
                    eprintln!("The value of the environment variable for `{candidate}` was not Unicode. Got {thing:?}");
                }
                (None, Ok(value)) => found = Some((candidate, value)),
                (Some((previous_key, previous_string)), Ok(value)) if *previous_string == value => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!("Redundant specification of the environment variable {candidate}, which was previously set via {previous_key}");
                    #[cfg(feature = "log")]
                    log::warn!("Redundant specification of the environment variable {candidate}, which was previously set via {previous_key}");
                    #[cfg(not(any(feature = "log", feature = "tracing")))]
                    eprintln!("Redundant specification of the environment variable {candidate}, which was previously set via {previous_key}");
                }
                (Some((previous_key, previous_string)), Ok(value)) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Inconsistent specification via environment variable {candidate}. Expected {previous_string} found {value}");
                    #[cfg(feature = "log")]
                    log::error!("Inconsistent specification via environment variable {candidate}. Expected {previous_string} found {value}");
                    #[cfg(not(any(feature = "log", feature = "tracing")))]
                    eprintln!("Inconsistent specification via environment variable {candidate}. Expected {previous_string} found {value}");
                    let err = super::Error::InconsistentSetting {
                        first_source: format!("Environment variable {previous_key}"),
                        first_setting: previous_string.clone(),
                        second_source: format!("Environment variable {candidate}"),
                        second_setting: value,
                    };
                    return Err(err);
                }
            }
        }
        Ok(found.map(|(_, value)| value))
    }
}

#[cfg(feature = "serde")]
pub mod serde_support {
    use super::{HasPartial, Source};

    #[cfg(feature = "toml")]
    use std::io::Read;

    /// Reading the file has failed. This will report the `Toml` and `Json` errors _if_ the
    /// appropriate features are enabled, so it is imperative that you do not forget to enable
    /// them.
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum FileReadError {
        /// Opening the file failed with the provided `io::Error`.
        Open(std::io::Error), // TODO: Implement proper `source` and other standard error traits.

        #[cfg(feature = "toml")]
        Toml(toml::de::Error), // TODO: Implement proper `soruce` and standard error trait methods. 

        #[cfg(feature = "json")]
        Json(serde_json::Error), // TODO: Implement proper `source` and standard eror trait
                                 // methods. 

        /// The file specified at this path does not exist. 
        NoFile(std::path::PathBuf),

        /// The file extension is not recognised. 
        UnsupportedExtension(String),

        /// The file has no extension. While UNIX supports files without extension, we do not
        /// believe that this is either sound reasoning or useful for many users. Just add `.toml`
        /// or provide a custom implementation if you really need to use files without extensions. 
        NoExtension,
    }

    impl From<std::io::Error> for FileReadError {
        fn from(value: std::io::Error) -> Self {
            Self::Open(value)
        }
    }

    impl core::fmt::Display for FileReadError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::NoExtension => {
                    write!(f, "No file extension provided. Aborting")
                }
                Self::UnsupportedExtension(s) => {
                    write!(f, "The file extension {s} is not supported")
                }
                Self::NoFile(path) => {
                    write!(f, "The file {path:?} could not be found")
                }
                Self::Open(err) => {
                    write!(f, "The file system reported the following error {err}")
                }
                #[cfg(feature = "toml")]
                Self::Toml(te) => {
                    write!(f, "Error parsing TOML file {te}")
                }
                #[cfg(feature = "json")]
                Self::Json(je) => {
                    write!(f, "Error parsing JSON file {je}")
                }
            }
        }
    }

    impl std::error::Error for FileReadError {}

    #[cfg(feature = "toml")]
    /// This is a strongly typed file with the TOML format and extension. Used for type checking.
    pub struct Toml<'a>(pub &'a std::path::Path);

    #[cfg(feature = "json")]
    /// This is a strongly typed file with the JSON format and extension. Used for type checking. 
    pub struct Json<'a>(pub &'a std::path::Path);

    #[cfg(feature = "json")]
    impl<'pth, C> Source<C> for Json<'pth>
    where
        C: HasPartial,
        C::Partial: serde::de::DeserializeOwned,
    {
        type Error = FileReadError;

        fn to_partial(self) -> Result<C::Partial, FileReadError> {
            let Self(path) = self;
            let file = std::fs::OpenOptions::new().read(true).open(path)?;
            let partial: C::Partial = serde_json::from_reader(file).map_err(FileReadError::Json)?;

            Ok(partial)
        }

        fn name(&self) -> String {
            format!("JSON file at {:?}", self.0)
        }
    }

    #[cfg(feature = "toml")]
    impl<'pth, C> Source<C> for Toml<'pth>
    where
        C: HasPartial,
        C::Partial: serde::de::DeserializeOwned,
    {
        type Error = FileReadError;

        fn to_partial(self) -> Result<C::Partial, FileReadError> {
            let Self(path) = self;
            let mut file = std::fs::OpenOptions::new().read(true).open(path)?;
            let mut buffer: String = String::new();
            file.read_to_string(&mut buffer)?;
            let partial: C::Partial = toml::from_str(&buffer).map_err(FileReadError::Toml)?;

            Ok(partial)
        }

        fn name(&self) -> String {
            format!("TOML file at {:?}", self.0)
        }
    }

    impl<C> Source<C> for std::path::PathBuf
    where
        C: HasPartial,
        C::Partial: serde::de::DeserializeOwned,
    {
        type Error = FileReadError;

        fn to_partial(self) -> Result<C::Partial, FileReadError> {
            if !self.exists() {
                Err(FileReadError::NoFile(self))
            } else {
                match self.extension() {
                    Some(os_str) => match os_str.to_str().expect("Failed conversion from OsStr") {
                        #[cfg(feature = "toml")]
                        "toml" | "tml" => <Toml<'_> as Source<C>>::to_partial(Toml(&self)),
                        #[cfg(feature = "json")]
                        "json" | "js" => <Json<'_> as Source<C>>::to_partial(Json(&self)),
                        rest => Err(FileReadError::UnsupportedExtension(rest.to_owned())),
                    },
                    None => Err(FileReadError::NoExtension),
                }
            }
        }

        fn name(&self) -> String {
            format!("Configuration file at `{:?}`", self)
        }
    }
}

/// Implement this trait if you want to indicate that your structure
/// can optionally contain a configuration path.
pub trait ConfigPath<T: AsRef<std::path::Path>> {
    /// Obtain a configuration path from `self`. Ideally you only want to consider things like
    /// `&str` but there can be other valid implementations.
    fn config_path(&self) -> Option<T>;
}
