use partial_config::HasPartial;

pub struct Optional;

#[derive(HasPartial)]
pub struct Configuration {
    /// This ise documented
    thing: u64,
    height: u64,
    thing2: Option<usize>,
}
