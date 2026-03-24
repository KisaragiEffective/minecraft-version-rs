use jiff::civil::DateTime;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::process::Stdio;

#[derive(Deserialize)]
struct VersionCollection {
    versions: Vec<Version>
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
            "https://launchermeta.mojang.com/mc/game/version_manifest.json"
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

    // avoid dirty tree
    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = std::path::Path::new(&out_dir).join("gen.rs");
    let f = File::options().create(true).write(true).append(false).truncate(true).open(&dest_path)?;

    let mut bw = BufWriter::new(f);
    use std::io::Write;

    let make_enum_name = |s: &str| format!("_{}", s.replace(['.', '-', ' '], "_"));
    let variants = versions
        .iter()
        .map(|x|
            format!("    {v},\n", v = make_enum_name(&x.id))
        )
        .collect::<Vec<_>>()
        .join("");

    let display_arms = versions
        .iter()
        .map(|x| format!("            Self::{name} => \"{value}\", \n", name = make_enum_name(&x.id), value = &x.id))
        .collect::<Vec<_>>()
        .join("");

    let from_str_arms = versions
        .iter()
        .map(|x| format!("            \"{ver}\" => Ok(Self::{variant}),\n", ver = &x.id, variant = make_enum_name(&x.id)))
        .collect::<Vec<_>>()
        .join("");

    // language=rust
    writeln!(&mut bw, r#"#[non_exhaustive]
#[allow(non_camel_case_types)]
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum MinecraftVersion {{
{variants}
}}

#[allow(unused_qualifications, clippy::too_many_lines)]
impl ::core::fmt::Display for MinecraftVersion {{
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {{
        let s = match self {{
{display_arms}
        }};

        f.write_str(s)
    }}
}}

#[allow(unused_qualifications, clippy::too_many_lines)]
impl ::core::str::FromStr for MinecraftVersion {{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {{
        match s {{
{from_str_arms}
            _ => ::core::result::Result::Err(())
        }}
    }}
}}"#)?;

    Ok(())
}
