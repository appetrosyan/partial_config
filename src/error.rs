//! Error types and variants used in this crate. They are provided for reference only, all traits
//! accept an optional `Error` type which you should make use of if you need customised errors.

/// A field that is required is not specified in _any_ of the layers. Missing from one layer is not
/// a hard error, and if you need that, you should consider using a different crate.
#[derive(Debug)]
pub struct MissingField<'a>(pub &'a str);

impl<'a> core::fmt::Display for MissingField<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The field {} is missing", self.0)
    }
}

impl<'a> std::error::Error for MissingField<'a> {}

/// All possible things that can go wrong when using `partial_config`.
#[derive(Debug)]
pub enum Error {
    /// Some of the required fields are missing
    MissingFields {
        // TODO: Consider using an array with fixed capacity and avoid allocation.
        required_fields: Vec<MissingField<'static>>,
    },
    /// A field that is supposed to be a number failed to be parsed from a string. Provided for
    /// convenience.
    ParseIntError(std::num::ParseIntError),
    /// A single setting was specified in two layers, and the two do not agree. This is useful in
    /// cases where you want to debug a _sticky_ setting that is strictly not supposed to be set,
    /// but is. It is also used for different aliases in the `EnvSourced` infrastructure of this
    /// crate.
    InconsistentSetting {
        first_source: String,
        first_setting: String,
        second_source: String,
        second_setting: String,
    },
    /// The field failed to parse. This is a more generic, and therefore less useful version of the
    /// other errors. Consider creating your own error type if you plan to do anything other than
    /// print the error message after this point.
    ParseFieldError {
        field_name: &'static str,
        field_type: &'static str,
        error_condition: Box<dyn std::error::Error + Send + Sync>,
    },
    #[cfg(feature = "serde")]
    /// The file failed to read.
    FileReadError(crate::serde_support::FileReadError),
    #[cfg(feature = "eyre")]
    /// This is a more sophisticated version of the `Box<dyn std::error::Error`, that can be
    /// cloned. Highly recommended but not required.
    EyreReport(eyre::Report),
}

#[cfg(feature = "serde")]
impl From<crate::serde_support::FileReadError> for Error {
    fn from(value: crate::serde_support::FileReadError) -> Self {
        Self::FileReadError(value)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

#[cfg(feature = "eyre")]
impl From<eyre::Report> for Error {
    fn from(value: eyre::Report) -> Self {
        Self::EyreReport(value)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingFields { required_fields } => {
                let fields: Vec<&str> = required_fields.iter().map(|field| field.0).collect();
                write!(f, "The required fields [{}] were not specified in any of the configuration sources", fields.join(", "))
            }
            Error::ParseIntError(per) => write!(f, "Failed to parse integer. {per}"),
            Error::InconsistentSetting {
                first_source,
                first_setting,
                second_source,
                second_setting,
            } => {
                write!(f, "The field was set twice first to {first_setting} in {first_source} and then a second time to {second_setting} in {second_source}")
            }
            Error::ParseFieldError {
                field_name,
                field_type,
                error_condition,
            } => {
                write!(f, "The field {field_name} failed to convert to {field_type}, because of {error_condition}")
            }
            #[cfg(feature = "eyre")]
            Error::EyreReport(report) => {
                write!(f, "{report:?}")
            }
            #[cfg(feature = "serde")]
            Error::FileReadError(err) => {
                write!(f, "File read error: `{}`", err)
            }
        }
    }
}

impl std::error::Error for Error {}
