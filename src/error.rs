#[derive(Debug)]
pub struct MissingField<'a>(pub &'a str);

impl<'a> core::fmt::Display for MissingField<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The field {} is missing", self.0)
    }
}

impl<'a> std::error::Error for MissingField<'a> {}

#[derive(Debug)]
pub enum Error {
    MissingFields {
        required_fields: Vec<MissingField<'static>>,
    },
    #[cfg(feature = "serde")]
    FileReadError(crate::serde_support::FileReadError),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingFields { required_fields } => {
                let fields: Vec<&str>= required_fields.iter().map(|field| {field.0}).collect();
                write!(f, "The required fields [{}] were not specified in any of the configuration sources", fields.join(", "))
            },
            #[cfg(feature = "serde")]
            Error::FileReadError(err) => {
                write!(f, "File read error: `{}`", err)
            }
        }
    }
}

impl std::error::Error for Error {}
