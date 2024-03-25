use partial_config::HasPartial;

pub struct Optional;

#[derive(HasPartial)]
pub struct Conf {

}

#[derive(HasPartial)]
pub struct Configuration {
    /// This is documented
    thing: u64,
    thing2: Option<usize>,
}
