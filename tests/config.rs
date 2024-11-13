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
