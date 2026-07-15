use partial_config::{EnvSourced, Error, HasPartial, Partial};

pub struct Optional;

pub type Height = Option<u64>;

#[derive(Debug, HasPartial, EnvSourced)]
pub struct Conf {
    #[env(THING1, THING2)]
    #[env(THING3)]
    pub str1: String,

    #[env(THING3)]
    pub obj2: usize,

    #[env(THING4)]
    pub option: Option<u64>,
}

#[derive(Clone, HasPartial, serde::Deserialize)]
#[partial_derives(Clone)]
#[partial_rename(CustomPartialConfiguration)]
pub struct Configuration {
    pub str1: &'static str,
    pub port: u64,
    pub height: Height,
    pub custom_struct: Str1OnlySource,
    pub optional_field: Option<u64>,
}

#[derive(Debug, Default)]
pub struct EnvVarSomething;

#[derive(Clone, Copy, Default, Debug, serde::Deserialize)]
pub struct Str1OnlySource;

impl partial_config::Source<Configuration> for Str1OnlySource {
    type Error = Error;

    fn to_partial(self) -> Result<<Configuration as HasPartial>::Partial, Self::Error> {
        Ok(CustomPartialConfiguration {
            str1: Some("CustomStruct"),
            port: None,
            height: None,
            custom_struct: None,
            optional_field: None,
        })
    }

    fn name(&self) -> String {
        "CustomStruct".to_owned()
    }
}

#[derive(Default, Debug, serde::Deserialize)]
pub struct OptionalOnlySource;

impl partial_config::Source<Configuration> for OptionalOnlySource {
    type Error = Error;

    fn to_partial(self) -> Result<<Configuration as HasPartial>::Partial, Self::Error> {
        Ok(CustomPartialConfiguration {
            str1: None,
            port: None,
            height: None,
            custom_struct: None,
            optional_field: Some(42),
        })
    }

    fn name(&self) -> String {
        "CustomStruct".to_owned()
    }
}

#[derive(Default, Debug, serde::Deserialize)]
pub struct DefaultSource;

impl partial_config::Source<Configuration> for DefaultSource {
    type Error = Error;

    fn to_partial(self) -> Result<<Configuration as HasPartial>::Partial, Self::Error> {
        Ok(CustomPartialConfiguration {
            str1: Some(Default::default()),
            port: Some(Default::default()),
            height: Some(Default::default()),
            custom_struct: Some(Default::default()),
            optional_field: None,
        })
    }

    fn name(&self) -> String {
        "DefaultSource".to_owned()
    }
}

#[test]
fn incomplete_config_fails_to_build() {
    let conf = CustomPartialConfiguration::default()
        .source(Str1OnlySource)
        .unwrap()
        .build();
    if let Err(Error::MissingFields { required_fields }) = conf {
        // One field was specified
        assert_eq!(required_fields.len(), 3);
    } else {
        panic!("This should have missing fields!");
    }
}

#[test]
fn complete_config_overrides_correctly() {
    let conf = CustomPartialConfiguration::default()
        .source(Str1OnlySource)
        .unwrap()
        .source(DefaultSource)
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(conf.str1, "".to_owned());
    let conf = CustomPartialConfiguration::default()
        .source(DefaultSource)
        .unwrap()
        .source(Str1OnlySource)
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(conf.str1, "CustomStruct".to_owned());

    let conf = CustomPartialConfiguration::default()
        .source(OptionalOnlySource)
        .unwrap()
        .source(DefaultSource)
        .unwrap()
        .build()
        .unwrap();
    assert_eq!(conf.optional_field, Some(42));
    let conf = CustomPartialConfiguration::default()
        .source(DefaultSource)
        .unwrap()
        .source(OptionalOnlySource)
        .unwrap()
        .clone()
        .build()
        .unwrap();
    assert_eq!(conf.optional_field, Some(42_u64));
    assert_eq!(conf.clone().optional_field, Some(42_u64))
}

#[test]
fn rename_works() {
    EnvVarSomething::default();
}

#[derive(Debug, HasPartial, EnvSourced)]
#[env_var_rename(SkipEnvSource)]
pub struct WithSkip {
    #[env(THING_A)]
    pub a: String,

    /// Operator intent, expressed on the command line — never read from the environment.
    #[env(skip)]
    pub b: String,
}

/// A `#[env(skip)]` field is not sourced from the environment: the generated source
/// leaves it `None` for the CLI and default layers to supply, and no same-named variable
/// can give it a value it was never meant to read.
#[test]
fn env_skip_field_is_never_sourced_from_the_environment() {
    std::env::set_var("THING_A", "from-env");
    std::env::set_var("B", "should-be-ignored");

    let partial =
        <SkipEnvSource as partial_config::Source<WithSkip>>::to_partial(SkipEnvSource::new())
            .unwrap();

    assert_eq!(partial.a, Some("from-env".to_string()));
    assert_eq!(
        partial.b, None,
        "a `#[env(skip)]` field must not be read from the environment"
    );

    std::env::remove_var("THING_A");
    std::env::remove_var("B");
}

#[derive(Debug, HasPartial, EnvSourced)]
#[env_var_rename(SecretEnvSource)]
pub struct WithSecret {
    #[env(APP_SECRET_TOKEN)]
    pub token: partial_config::Redacted<String>,
}

/// A `Redacted<T>` field is an ordinary env-sourced field — parsed and wrapped — and the
/// wrapper keeps it out of the partial's `Debug`, so it cannot leak when the configuration
/// is logged.
#[test]
fn a_redacted_field_sources_from_the_environment_and_stays_hidden() {
    std::env::set_var("APP_SECRET_TOKEN", "s3cr3t-value");

    let partial =
        <SecretEnvSource as partial_config::Source<WithSecret>>::to_partial(SecretEnvSource::new())
            .unwrap();

    assert_eq!(
        partial.token.as_ref().map(|t| t.expose_secret().as_str()),
        Some("s3cr3t-value"),
        "the value is sourced into the wrapper"
    );
    assert!(
        !format!("{:?}", partial.token).contains("s3cr3t-value"),
        "and the wrapper hides it from Debug"
    );

    std::env::remove_var("APP_SECRET_TOKEN");
}
