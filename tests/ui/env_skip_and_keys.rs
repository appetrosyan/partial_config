// A field cannot both name environment variables and opt out with `skip`.

#[derive(partial_config::HasPartial, partial_config::EnvSourced)]
pub struct Configuration {
    #[env(FOO, skip)]
    pub field: String,
}

fn main() {}
