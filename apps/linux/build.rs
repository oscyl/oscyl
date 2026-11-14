//! Bundles everything under `data/` (stylesheets, icons, the font, the
//! third-party licences) into a GResource, plus the token stylesheets generated from the shared
//! `design/tokens.json`. The manifest is generated so new icons need no
//! bookkeeping.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const PREFIX: &str = "/io/github/oscyl/Oscyl";

fn collect(dir: &Path, root: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect(&path, root, out);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("css" | "svg" | "ttf" | "json")
        ) {
            let rel = path.strip_prefix(root).expect("under data/");
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// One stylesheet per colour mode (`tokens-light.css`, `tokens-dark.css`,
/// `tokens-amoled.css`): every colour role as `--oscyl-<role>`, plus shape
/// and motion in the light (base) sheet.
fn generate_tokens(source: &Path, out: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(source).expect("read design/tokens.json");
    let tokens: serde_json::Value = serde_json::from_str(&text).expect("tokens.json is JSON");
    let modes = tokens["color"].as_object().expect("color modes");
    let mut files = Vec::new();
    for (mode, roles) in modes {
        let mut css =
            format!("/* Generated from design/tokens.json ({mode}). Do not edit. */\n:root {{\n");
        for (role, value) in roles.as_object().expect("colour roles") {
            let value = value.as_str().expect("colour value");
            writeln!(css, "  --oscyl-{role}: {value};").expect("write to string");
        }
        if mode == "light" {
            let shape = &tokens["shape"];
            let motion = &tokens["motion"];
            for (name, value) in [
                ("radius", format!("{}px", shape["radius_px"])),
                ("radius-inner", format!("{}px", shape["radius_inner_px"])),
                ("radius-panel", format!("{}px", shape["radius_panel_px"])),
                ("rule-width", format!("{}px", shape["rule_px"])),
                ("fast", format!("{}ms", motion["fast_ms"])),
                ("base", format!("{}ms", motion["base_ms"])),
                ("slow", format!("{}ms", motion["slow_ms"])),
                (
                    "ease",
                    motion["easing"].as_str().expect("easing").to_string(),
                ),
            ] {
                writeln!(css, "  --oscyl-{name}: {value};").expect("write to string");
            }
        }
        css.push_str("}\n");
        let name = format!("tokens-{mode}.css");
        std::fs::write(out.join(&name), css).expect("write token stylesheet");
        files.push(name);
    }
    files
}

fn main() {
    let data = PathBuf::from("data");
    let tokens = PathBuf::from("../../design/tokens.json");
    println!("cargo:rerun-if-changed=data");
    println!("cargo:rerun-if-changed={}", tokens.display());
    let mut files = Vec::new();
    collect(&data, &data, &mut files);
    let generated = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("generated");
    std::fs::create_dir_all(&generated).expect("create generated dir");
    files.extend(generate_tokens(&tokens, &generated));

    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gresources>\n  <gresource prefix=\"{PREFIX}\">\n"
    );
    for f in &files {
        let compress = if f.ends_with(".ttf") {
            ""
        } else {
            " compressed=\"true\""
        };
        writeln!(xml, "    <file{compress}>{f}</file>").expect("write to string");
    }
    xml.push_str("  </gresource>\n</gresources>\n");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let manifest = out.join("resources.gresource.xml");
    std::fs::write(&manifest, xml).expect("write gresource manifest");
    glib_build_tools::compile_resources(
        &[
            data.to_str().expect("utf-8 path"),
            generated.to_str().expect("utf-8 path"),
        ],
        manifest.to_str().expect("utf-8 path"),
        "oscyl.gresource",
    );
}
