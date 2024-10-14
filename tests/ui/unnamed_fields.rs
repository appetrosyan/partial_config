use partial_config::HasPartial;

#[derive(Debug, HasPartial)]
pub struct Conf(Option<u64>);

#[derive(HasPartial)]
pub struct Config;

fn main() {}
