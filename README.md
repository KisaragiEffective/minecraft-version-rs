# minecraft-version

An enum of Minecraft versions generated from Mojang's official version manifest at build time.

## Usage

```rust
use minecraft_version::MinecraftVersion;
use std::str::FromStr;

fn main() {
    let my_version = MinecraftVersion::from_str("1.19.3").unwrap();
    println!("Your selected version is {}", my_version);
}
```

## Building

### Requirements

The `curl` command must be available in your PATH to build this crate. No additional curl development headers or libraries are required.

This crate uses the `curl` command instead of a Rust HTTP client to avoid pulling in additional TLS dependencies.

### How it works

At build time, this crate fetches the [Minecraft version manifest](https://launchermeta.mojang.com/mc/game/version_manifest.json) from Mojang's API (see the [Minecraft Wiki](https://minecraft.wiki/w/Mojang_API)) and generates a Rust enum containing the listed versions.

## License

Licensed under either the Apache License, Version 2.0 or the MIT license, at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in minecraft-version by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
