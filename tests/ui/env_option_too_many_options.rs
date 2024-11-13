#[derive(partial_config::HasPartial, partial_config::EnvSourced)]
pub struct Configuration {
    #[env(OPTIONAL)]
    pub optional: Option<Option<String>>
}

fn main()  {}
