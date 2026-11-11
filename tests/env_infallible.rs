#![deny(unreachable_code)]

use partial_config::{EnvSourced, Error, HasPartial, Redacted, Source};
use std::convert::Infallible;
use std::str::FromStr;

#[derive(Debug, Default)]
pub struct InfallibleValue(String);

impl FromStr for InfallibleValue {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.to_owned()))
    }
}

#[derive(Debug, HasPartial, EnvSourced)]
pub struct Configuration {
    #[env(INFALLIBLE_REQUIRED)]
    pub required: Redacted<String>,
    #[env(INFALLIBLE_OPTIONAL)]
    pub optional: Option<Redacted<String>>,
    #[env(INFALLIBLE_CUSTOM)]
    pub custom: InfallibleValue,
    #[env(INFALLIBLE_PORT)]
    pub port: u16,
}

#[test]
fn infallible_required_optional_and_custom_fields_are_sourced() {
    std::env::set_var("PC_INFALLIBLE_REQUIRED_OK", "required secret");
    std::env::set_var("PC_INFALLIBLE_OPTIONAL_OK", "optional secret");
    std::env::set_var("PC_INFALLIBLE_CUSTOM_OK", "custom value");
    std::env::set_var("PC_INFALLIBLE_PORT_OK", "8080");
    let partial = EnvVarSource {
        required: ["PC_INFALLIBLE_REQUIRED_OK"],
        optional: ["PC_INFALLIBLE_OPTIONAL_OK"],
        custom: ["PC_INFALLIBLE_CUSTOM_OK"],
        port: ["PC_INFALLIBLE_PORT_OK"],
    }
    .to_partial()
    .expect("infallible fields and a valid port must parse");

    assert_eq!(
        partial.required.expect("required value").expose_secret(),
        "required secret"
    );
    assert_eq!(
        partial.optional.expect("optional value").expose_secret(),
        "optional secret"
    );
    assert_eq!(partial.custom.expect("custom value").0, "custom value");
    assert_eq!(partial.port, Some(8080));
}

#[test]
fn fallible_fields_keep_their_error_context_and_concrete_error() {
    std::env::set_var("PC_INFALLIBLE_PORT_ERROR", "not a port");
    let result = EnvVarSource {
        required: ["PC_INFALLIBLE_REQUIRED_ERROR"],
        optional: ["PC_INFALLIBLE_OPTIONAL_ERROR"],
        custom: ["PC_INFALLIBLE_CUSTOM_ERROR"],
        port: ["PC_INFALLIBLE_PORT_ERROR"],
    }
    .to_partial();

    match result {
        Err(Error::ParseFieldError {
            field_name,
            field_type,
            error_condition,
        }) => {
            assert_eq!(field_name, "port");
            assert_eq!(field_type, "u16");
            assert!(error_condition
                .downcast_ref::<std::num::ParseIntError>()
                .is_some());
        }
        _ => panic!("invalid port must retain its named parse error"),
    }
}
