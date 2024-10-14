#[derive(partial_config::HasPartial)]
#[repr(C)]
pub union Conf {
    f1: u32,
    f2: f64,
}

fn main() {}
