use jiff::civil::DateTime;
use quote::quote;
use serde::Deserialize;
use std::error::Error;
use std::process::Stdio;

#[derive(Deserialize)]
struct VersionCollection {
    versions: Vec<Version>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Version {
    id: String,
    release_time: DateTime,
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // hack: avoid large crate trees (bloating by rustls - reducing about 250MB).
    // Even if on Windows, 1804 and later includes `curl` on $PATH-searchable directory.
    // CreateProcessW adds the `.exe` suffix, so `curl` program can be justified in most environments.
    let output = std::process::Command::new("curl")
        .args([
            "--compressed",
            "--fail",
            "--location",
            "--user-agent",
            r#""minecraft-version-rs/build-script (+https://crates.io/crates/minecraft-version)""#,
            "https://launchermeta.mojang.com/mc/game/version_manifest.json",
        ])
        .stdout(Stdio::piped())
        .output()
        .expect("curl is required to build this crate");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed to fetch version manifest: {stderr}").into());
    }

    let stdout = output.stdout;

    let mut raw_versions = serde_json::from_slice::<VersionCollection>(&stdout)?.versions;

    raw_versions.sort_by_key(|x| x.release_time);

    let variants = raw_versions.iter()
        .map(|v| {
            let mut mangled_version = v.id.replace(['.', '-', ' '], "_");
            mangled_version.insert(0, '_');

            proc_macro2::Ident::new(&mangled_version, proc_macro2::Span::call_site())
        })
        .collect::<Vec<_>>();

    // `quote!`-ed tokens does not have to be collected into Vec: the macro accepts any `Iterator`.
    let version_strings = variants
        .iter()
        .enumerate()
        .map(|(i, version)| {
            let s = &raw_versions[i].id;

            quote! {Self::#version => #s}
        });

    let strings_versions = variants
        .iter()
        .enumerate()
        .map(|(i, version)| {
            let s = &raw_versions[i].id;

            quote! {#s => Ok(Self::#version)}
        });


    let tokens = quote! {
        #[non_exhaustive]
        #[allow(non_camel_case_types)]
        #[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
        pub enum MinecraftVersion {
            #(#variants),*
        }

        impl core::fmt::Display for MinecraftVersion {
            #[allow(clippy::too_many_lines)]
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let s = match self {
                    #(#version_strings),*
                };
                f.write_str(s)
            }
        }
        impl core::str::FromStr for MinecraftVersion {
            type Err = ();
            #[allow(clippy::too_many_lines)]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(#strings_versions),*,
                    _ => Err(())
                }
            }
        }
    };

    // avoid dirty tree
    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = std::path::Path::new(&out_dir).join("gen.rs");

    let syntax_tree = syn::parse2(tokens)?;
    std::fs::write(dest_path, prettyplease::unparse(&syntax_tree))?;

    println!("cargo::rerun-if-changed=build.rs");
    Ok(())
}
