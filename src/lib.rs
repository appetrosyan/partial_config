use core::fmt::Debug;
mod error;
pub use error::{Error, MissingField};

#[cfg(feature = "derive")]
pub use partial_config_derive::HasPartial;

pub trait Partial: Default {
    type Target: HasPartial<Partial = Self>;

    type Error: Debug;

    fn build(self) -> Result<Self::Target, Self::Error>;

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

    fn override_with(self, other: Self) -> Self;
}

pub trait HasPartial {
    type Partial: Partial<Target = Self>;
}

pub trait Source<C: HasPartial> {
    type Error: Debug;

    fn to_partial(self) -> Result<C::Partial, Self::Error>;

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

#[cfg(feature = "serde")]
pub mod serde_support {
    use super::{HasPartial, Source};

    #[cfg(feature = "toml")]
    use std::io::Read;

    #[derive(Debug)]
    #[non_exhaustive]
    pub enum FileReadError {
        Open(std::io::Error),

        #[cfg(feature = "toml")]
        Toml(toml::de::Error),

        #[cfg(feature = "json")]
        Json(serde_json::Error),

        NoFile(std::path::PathBuf),

        UnsupportedExtension(String),

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

    #[cfg(feature = "toml")]
    pub struct Toml<'a>(pub &'a std::path::Path);

    #[cfg(feature = "json")]
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
                    Some(os_str) => match os_str.to_str().expect("Failed convrsion from OsStr") {
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
