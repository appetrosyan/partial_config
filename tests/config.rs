use partial_config::HasPartial;

pub struct Optional;

#[derive(Default, Debug)]
pub struct CustomStruct;

pub type Height = Option<u64>;

#[derive(HasPartial)]
pub struct Configuration {
    pub str1: String,
    pub port: u64,
    pub height: Height,
    pub custom_struct: CustomStruct,
}
