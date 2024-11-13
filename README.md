# Partial Config

This is a WIP crate for providing a generic interface to configure one's application.

It is typical to see a configuration be composed from multiple sources: the command line, the environment variables, a config file, sometimes even through a web server.

This crate provides a generic way to do so.

## The `Partial` trait

Imagine that you have a web service, that also needs to be configurable at runtime.  The solution is simple, create a layered structure with `Option`s everywhere that can be built from a different sources, and collapse them into one.  This trait and the corresponding derive macro `HasPartial` do that work for you.

The trait gives you three functions: `build` collapses a partial layer into a complete structure, the `<Self as Partial>::Target`, and reports errors such as conversion failures, required fields that are missing, and does so **properly**, for example, if you have multiple missing fields, they will all be reported _at once_ as opposed to one-by-one.

Then you have `override_with`, used like you would expect:
```rust
bottom_layer.override_with(top_layer);
```
it takes two layers, and applies fields specified in the `top_layer` overriding the `bottom_layer`, if present.  Fields absent in `top_layer` are inherited from the `bottom_layer`.  You'd be surprised how many times have I had to fix this logic.

Finally you get `source`, which we shall talk about later.

## The `HasPartial` derive macro

Say you have a configuration structure:

```rust
pub struct Configuration {
	file_name: String,
	port: u16,
	// Many fields...
	configuration_file: Option<String>,
}
```

You created a partial layer, that keeps track of which fields are required (_e.g._ `file_name`, `port`) and which fields are optional: `configuration_file`.  If you add or rename a new field, you would need to track that change into the layered structure, and if obtaining the layer is done via some other mechanism, _e.g._ `serde` from a `toml` file, keeping the two in sync is a lot of work.  Fortunately, `partial_config::HasPartial` can generate the layered structure for you.

You can forward derive annotations with `#[partial_derives(serde::Deserialize)]` in case your intermediate layers need to implement a trait, and doing so manually is too much work.  By default the partial layer is called `Partial<YourStructName>`, but it can be changed with the `#[partial_rename(CustomNameForYourIntermediateLayer)]` annotation.

## Source(s)

This is the main attraction of this package.  If you implement `Source<Configuration>` you now have access to the wonderful `source` method in the structure's partial representation.  This allows you to do what _ought_ to be simple for a CLI application to be genuinely simple:

```rust
let configuration = PartialConfiguration::default()
	.source("default_config.toml")?
	.source(EnvVars::new())?
	.source(Args::parse())?
	.build()?;
```

This does what you think it does, and in the order that you think it does: the configuration file has lowest priority, environment variables override that, and CLI arguments override everything.  Useful for when you're testing your program inside docker.

So how do you implement `Source`?  That's the neat part!

### `serde`

If you want a quick and dirty way to obtain fields from a configuration file, just `derive(serde::Deserialize)` on the `Configuration` and you get `source("path_to.toml")` for free.  
