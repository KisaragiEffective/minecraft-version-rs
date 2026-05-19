use jiff::civil::DateTime;
use quote::quote;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
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

    let mut versions = serde_json::from_slice::<VersionCollection>(&stdout)?.versions;

    versions.sort_by_key(|x| x.release_time);

    let variants = versions
        .iter()
        .map(|version| {
            let mut id = version.id.replace(['.', '-', ' '], "_");
            id.insert(0, '_');
            let id: proc_macro2::TokenStream = id.parse().unwrap();
            quote! {#id}
        })
        .collect::<Vec<_>>();

    let version_strings = versions
        .iter()
        .map(|version| {
            let mut id = version.id.replace(['.', '-', ' '], "_");
            id.insert(0, '_');
            let id: proc_macro2::TokenStream = id.parse().unwrap();
            let id2 = version.id.clone();
            quote! {Self::#id => #id2}
        })
        .collect::<Vec<_>>();

    let strings_versions = versions
        .iter()
        .map(|version| {
            let mut id = version.id.replace(['.', '-', ' '], "_");
            id.insert(0, '_');
            let id: proc_macro2::TokenStream = id.parse().unwrap();
            let id2 = version.id.clone();
            quote! {#id2=>  Ok(Self::#id)}
        })
        .collect::<Vec<_>>();

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
    let f = File::options()
        .create(true)
        .write(true)
        .append(false)
        .truncate(true)
        .open(dest_path)?;
    let mut bw = BufWriter::new(f);
    write!(&mut bw, "{}", prettyplease::unparse(&syntax_tree))?;

    println!("cargo::rerun-if-changed=build.rs");
    Ok(())
}
