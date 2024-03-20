pub trait Partial: Default {
    type Target: HasPartial<Partial = Self>;

    type Error;

    fn build(self) -> Result<Self::Target, Self::Error>;

    fn source(self, value: impl Source<Self::Target>) -> Result<Self::Target, Self::Error>;

    fn override_with(self, other: Self) -> Self;
}

pub trait HasPartial {
    type Partial: Partial<Target = Self>;
}

pub trait Source<C: HasPartial> {
    type Error;

    fn to_partial(self) -> Result<C::Partial, Self::Error>;

    fn name(&self) -> &str;
}

impl<T, C, E> Source<C> for Option<T>
where
    C: HasPartial,
    T: Source<C, Error = E>,
{
    type Error = E;

    fn to_partial(self) -> Result<C::Partial, E> {
        self.map_or_else(
            || Ok(C::Partial::default()),
            |v| v.to_partial()
        )
    }
    
    fn name(&self) -> &str {
        self.as_ref().map_or(
            "Unspecified",
            |v| v.name(),
        )
    }
}
