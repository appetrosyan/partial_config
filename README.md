# Partial Config

This is a WIP crate for providing a generic interface to configure one's application.

It is typical to see a configuration be composed from multiple sources: the command line, the environment variables, a config file, sometimes even through a web server.

This crate provides a generic way to do so.

Specifically, it provides
- [X] Two traits `Partial` and `Source`
- [X] A derive macro `Partial` that generates a new structure
- [X] An implementation for `Source` if your configuration also is `serde::de::DeserializeOwned`
- [X] Logic to combine multiple configuration sources
- [X] Derive macro to produce detailed error reports
- [X] Support for logging overlapping or overriding specifications
