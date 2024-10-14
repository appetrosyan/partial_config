
#[derive(partial_config::HasPartial)]
pub struct LifetimeConfiguration<'a> {
    string1: &'a str,
}

fn main() {}
