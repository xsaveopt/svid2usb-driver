use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail, ensure};
use clap::{Parser, Subcommand};

const OBS_VERSION: &str = "32.2.2";
const OBS_SHA256: &str = "35d3cd0979d65664fada7119fdb612eca7c34b61a1623a330caec74bf72626c4";
const SIMDE_VERSION: &str = "0.8.2";
const SIMDE_SHA256: &str = "ed2a3268658f2f2a9b5367628a85ccd4cf9516460ed8604eed369653d49b25fb";
const PLUGIN: &str = "svid2usb";
const BUNDLE_ID: &str = "io.github.svid2usb";
const UNIVERSAL: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

#[derive(Parser)]
#[command(about = "Development tasks for svid2usb")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    #[command(about = "Regenerate the libobs FFI bindings from the pinned OBS headers")]
    Bindings,
    #[command(about = "Build svid2usb.plugin for this Mac into target/bundle")]
    Bundle,
    #[command(about = "Build svid2usb.plugin and install it into the OBS plugins folder")]
    Install,
    #[command(about = "Build a universal (Apple silicon + Intel) release zip into target/dist")]
    Dist {
        #[arg(long, default_value = env!("CARGO_PKG_VERSION"))]
        version: String,
    },
}

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("xtask has no parent directory")?
        .to_path_buf();
    match Cli::parse().task {
        Task::Bindings => bindings(&root),
        Task::Bundle => bundle(&root).map(|dir| println!("{}", dir.display())),
        Task::Install => install(&root),
        Task::Dist { version } => dist(&root, &version).map(|zip| println!("{}", zip.display())),
    }
}

fn run(cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("running {cmd:?}"))?;
    ensure!(status.success(), "{cmd:?} failed with {status}");
    Ok(())
}

fn output(cmd: &mut Command) -> Result<String> {
    let out = cmd.output().with_context(|| format!("running {cmd:?}"))?;
    ensure!(out.status.success(), "{cmd:?} failed with {}", out.status);
    Ok(String::from_utf8(out.stdout)?)
}

fn fetch(work: &Path, name: &str, url: &str, sha256: &str) -> Result<PathBuf> {
    let dir = work.join(name);
    if dir.is_dir() {
        return Ok(dir);
    }
    fs::create_dir_all(work)?;
    let archive = work.join(format!("{name}.tar.gz"));
    run(Command::new("curl").args(["-fsSL", "-o"]).arg(&archive).arg(url))?;
    let sum = output(Command::new("shasum").args(["-a", "256"]).arg(&archive))?;
    ensure!(sum.starts_with(sha256), "checksum mismatch for {url}");
    fs::create_dir_all(&dir)?;
    run(Command::new("tar")
        .args(["-xzf"])
        .arg(&archive)
        .args(["--strip-components=1", "-C"])
        .arg(&dir))?;
    fs::remove_file(&archive)?;
    Ok(dir)
}

fn bindings(root: &Path) -> Result<()> {
    let work = root.join("target/xtask");
    let obs = fetch(
        &work,
        &format!("obs-studio-{OBS_VERSION}"),
        &format!("https://github.com/obsproject/obs-studio/archive/refs/tags/{OBS_VERSION}.tar.gz"),
        OBS_SHA256,
    )?;
    let simde = fetch(
        &work,
        &format!("simde-{SIMDE_VERSION}"),
        &format!("https://github.com/simd-everywhere/simde/archive/refs/tags/v{SIMDE_VERSION}.tar.gz"),
        SIMDE_SHA256,
    )?;
    let generated = work.join("generated");
    fs::create_dir_all(&generated)?;
    fs::write(
        generated.join("obsconfig.h"),
        "#pragma once\n#define OBS_RELEASE_CANDIDATE 0\n#define OBS_BETA 0\n",
    )?;
    fs::write(
        generated.join("wrapper.h"),
        "#include <obs-module.h>\n#include <util/platform.h>\n",
    )?;

    let out = root.join("crates/svid2usb/src/obs_sys.rs");
    bindgen::Builder::default()
        .header(generated.join("wrapper.h").to_string_lossy())
        .clang_arg(format!("-I{}", obs.join("libobs").display()))
        .clang_arg(format!("-I{}", simde.display()))
        .clang_arg(format!("-I{}", generated.display()))
        .allowlist_function(
            "obs_register_source_s|obs_source_output_video|obs_data_get_int|obs_data_set_default_int|\
             obs_properties_create|obs_properties_add_list|obs_property_list_add_int|\
             obs_properties_add_int_slider|obs_property_set_modified_callback|\
             blog|os_gettime_ns|video_format_get_parameters_for_format",
        )
        .allowlist_type("obs_source_info|obs_source_frame")
        .allowlist_var(
            "LIBOBS_API_(MAJOR|MINOR|PATCH)_VER|LOG_(ERROR|WARNING|INFO)|OBS_SOURCE_(ASYNC_VIDEO|DO_NOT_DUPLICATE)",
        )
        .default_enum_style(bindgen::EnumVariation::ModuleConsts)
        .derive_default(true)
        .generate_comments(false)
        .disable_header_comment()
        .rust_target(bindgen::RustTarget::stable(85, 0).map_err(|e| anyhow!("{e}"))?)
        .rust_edition(bindgen::RustEdition::Edition2024)
        .generate()
        .context("generating libobs bindings")?
        .write_to_file(&out)?;
    run(Command::new("rustfmt")
        .args(["--edition", "2024", "--config-path"])
        .arg(root.join("rustfmt.toml"))
        .arg(&out))?;
    println!("{}", out.display());
    Ok(())
}

fn dylib_path(root: &Path, target: Option<&str>) -> PathBuf {
    let mut dir = root.join("target");
    if let Some(target) = target {
        dir.push(target);
    }
    dir.join("release").join(format!("lib{PLUGIN}.dylib"))
}

fn cargo_build(root: &Path, target: Option<&str>, version: &str) -> Result<PathBuf> {
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(root)
        .args(["build", "--release", "--package", PLUGIN])
        .env("SVID2USB_VERSION", version);
    if let Some(target) = target {
        cmd.args(["--target", target]);
    }
    run(&mut cmd)?;
    Ok(dylib_path(root, target))
}

fn merge(parts: &[PathBuf], dest: &Path) -> Result<()> {
    if let [single] = parts {
        fs::copy(single, dest)?;
    } else {
        run(Command::new("lipo").arg("-create").args(parts).arg("-output").arg(dest))?;
    }
    Ok(())
}

fn info_plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>{PLUGIN}</string>
	<key>CFBundleIdentifier</key>
	<string>{BUNDLE_ID}</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>{PLUGIN}</string>
	<key>CFBundlePackageType</key>
	<string>BNDL</string>
	<key>CFBundleShortVersionString</key>
	<string>{version}</string>
	<key>CFBundleVersion</key>
	<string>{version}</string>
</dict>
</plist>
"#
    )
}

struct Layout {
    plugin: PathBuf,
    info_plist: PathBuf,
    executable: PathBuf,
}

fn layout(dir: &Path) -> Layout {
    let plugin = dir.join(format!("{PLUGIN}.plugin"));
    Layout {
        info_plist: plugin.join("Contents/Info.plist"),
        executable: plugin.join("Contents/MacOS").join(PLUGIN),
        plugin,
    }
}

fn assemble(dir: &Path, builds: &[PathBuf], version: &str) -> Result<()> {
    if dir.exists() {
        fs::remove_dir_all(dir)?;
    }
    let bundle = layout(dir);
    for parent in [&bundle.info_plist, &bundle.executable] {
        fs::create_dir_all(parent.parent().context("bundle paths have a parent")?)?;
    }
    fs::write(&bundle.info_plist, info_plist(version))?;
    merge(builds, &bundle.executable)?;
    run(Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(&bundle.plugin))
}

fn bundle(root: &Path) -> Result<PathBuf> {
    let version = env!("CARGO_PKG_VERSION");
    let built = cargo_build(root, None, version)?;
    let dir = root.join("target/bundle");
    assemble(&dir, &[built], version)?;
    Ok(dir)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn plugins_folder(home: &Path) -> PathBuf {
    home.join("Library/Application Support/obs-studio/plugins")
}

fn install(root: &Path) -> Result<()> {
    let dir = bundle(root)?;
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    let dest = layout(&plugins_folder(Path::new(&home))).plugin;
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    copy_dir(&layout(&dir).plugin, &dest)?;
    println!("installed {}", dest.display());
    Ok(())
}

fn dist_name(version: &str) -> Result<String> {
    if version.contains('/') || version.is_empty() {
        bail!("invalid version '{version}'");
    }
    Ok(format!("{PLUGIN}-{version}"))
}

fn archive_name(name: &str) -> String {
    format!("{name}-macos-universal.zip")
}

fn dist(root: &Path, version: &str) -> Result<PathBuf> {
    let name = dist_name(version)?;
    let builds = UNIVERSAL
        .iter()
        .map(|target| cargo_build(root, Some(target), version))
        .collect::<Result<Vec<_>>>()?;
    let dist = root.join("target/dist");
    let dir = dist.join(&name);
    assemble(&dir, &builds, version)?;
    fs::copy(root.join("LICENSE"), dir.join("LICENSE"))?;
    fs::copy(root.join("README.md"), dir.join("README.md"))?;
    let zip = dist.join(archive_name(&name));
    if zip.exists() {
        fs::remove_file(&zip)?;
    }
    run(Command::new("ditto")
        .args(["-c", "-k", "--keepParent"])
        .arg(&dir)
        .arg(&zip))?;
    Ok(zip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_build_lands_in_the_plain_release_folder() {
        let root = Path::new("/w/svid2usb-driver");
        assert_eq!(
            dylib_path(root, None),
            PathBuf::from("/w/svid2usb-driver/target/release/libsvid2usb.dylib")
        );
    }

    #[test]
    fn a_cross_build_lands_under_its_target_triple() {
        let root = Path::new("/w/svid2usb-driver");
        let paths: Vec<PathBuf> = UNIVERSAL.iter().map(|t| dylib_path(root, Some(t))).collect();
        assert_eq!(
            paths,
            [
                PathBuf::from("/w/svid2usb-driver/target/aarch64-apple-darwin/release/libsvid2usb.dylib"),
                PathBuf::from("/w/svid2usb-driver/target/x86_64-apple-darwin/release/libsvid2usb.dylib"),
            ]
        );
    }

    #[test]
    fn the_bundle_is_a_macos_plugin_layout() {
        let bundle = layout(Path::new("/w/target/bundle"));
        assert_eq!(bundle.plugin, PathBuf::from("/w/target/bundle/svid2usb.plugin"));
        assert_eq!(
            bundle.info_plist,
            PathBuf::from("/w/target/bundle/svid2usb.plugin/Contents/Info.plist")
        );
        assert_eq!(
            bundle.executable,
            PathBuf::from("/w/target/bundle/svid2usb.plugin/Contents/MacOS/svid2usb")
        );
        assert_eq!(
            bundle.executable.parent(),
            Some(bundle.plugin.join("Contents/MacOS").as_path())
        );
    }

    #[test]
    fn the_executable_is_named_after_the_bundle() {
        let bundle = layout(Path::new("relative"));
        assert_eq!(bundle.executable.file_name().unwrap(), PLUGIN);
        assert_eq!(bundle.plugin.file_name().unwrap(), format!("{PLUGIN}.plugin").as_str());
    }

    #[test]
    fn plugins_go_into_the_obs_support_folder() {
        assert_eq!(
            layout(&plugins_folder(Path::new("/Users/someone"))).plugin,
            PathBuf::from("/Users/someone/Library/Application Support/obs-studio/plugins/svid2usb.plugin")
        );
    }

    #[test]
    fn a_distribution_is_named_after_the_plugin_and_version() {
        assert_eq!(dist_name("1.2.3").unwrap(), "svid2usb-1.2.3");
        assert_eq!(dist_name("dev-abc1234").unwrap(), "svid2usb-dev-abc1234");
        assert_eq!(archive_name("svid2usb-1.2.3"), "svid2usb-1.2.3-macos-universal.zip");
    }

    #[test]
    fn a_version_that_could_escape_the_dist_folder_is_refused() {
        for version in ["", "1.0/../../etc", "/absolute"] {
            assert!(dist_name(version).is_err(), "{version} should be refused");
        }
    }

    #[test]
    fn the_plist_carries_the_version_and_the_bundle_identity() {
        let plist = info_plist("1.2.3");
        assert!(plist.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
        assert!(plist.contains("<key>CFBundleIdentifier</key>\n\t<string>io.github.svid2usb</string>"));
        assert!(plist.contains("<key>CFBundleExecutable</key>\n\t<string>svid2usb</string>"));
        assert!(plist.contains("<key>CFBundleShortVersionString</key>\n\t<string>1.2.3</string>"));
        assert!(plist.contains("<key>CFBundleVersion</key>\n\t<string>1.2.3</string>"));
        assert!(plist.trim_end().ends_with("</plist>"));
        assert!(!plist.contains('{'), "no format placeholder is left behind");
    }
}
