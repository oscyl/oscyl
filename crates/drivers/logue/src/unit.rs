//! logue SDK unit files (`.mnlgxdunit`, `.prlgunit`) and the slot data the
//! synth stores.
//!
//! A unit file is a ZIP holding `<name>/manifest.json` and
//! `<name>/payload.bin`. The synth stores a blob of
//! `header (0x400) + payload + signature area (0x84 zero bytes)`, where the
//! header is the SDK's `user_prg_header_t` built from the manifest. Layout
//! confirmed against a unit read back from an xd
//! (a user slot's data, read back); the
//! prologue's bundled unit file has the same layout.

use std::io::{Read, Seek};

use serde_json::Value;
use zip::ZipArchive;

use crate::Error;
use crate::msg::Module;

pub const HEADER_LEN: usize = 0x400;
pub const SIG_LEN: usize = 0x84;
const MAX_PARAMS: usize = 6;
const NAME_LEN: usize = 13;
const PARAM_NAME_LEN: usize = 12;

/// A logue SDK platform (`userprg.h`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    /// As a manifest spells it: `minilogue-xd`, `prologue`.
    pub name: &'static str,
    /// prologue 1, minilogue xd 2.
    pub id: u8,
}

/// A parsed unit file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub module: Module,
    pub name: String,
    pub header: Vec<u8>,
    pub payload: Vec<u8>,
}

fn bad(msg: impl Into<String>) -> Error {
    Error::NotALibrary(msg.into())
}

/// Parse "M.m-p" into `M << 16 | m << 8 | p`.
fn version(s: &str) -> Result<u32, Error> {
    let (mm, p) = s
        .split_once('-')
        .ok_or_else(|| bad(format!("bad version {s:?}")))?;
    let (ma, mi) = mm
        .split_once('.')
        .ok_or_else(|| bad(format!("bad version {s:?}")))?;
    let n = |x: &str| {
        x.trim()
            .parse::<u8>()
            .map_err(|_| bad(format!("bad version {s:?}")))
    };
    Ok((n(ma)? as u32) << 16 | (n(mi)? as u32) << 8 | n(p)? as u32)
}

fn put_str(dst: &mut [u8], s: &str, max: usize) -> Result<(), Error> {
    if !s.is_ascii() || s.len() > max {
        return Err(bad(format!(
            "name {s:?} must be ASCII, at most {max} characters"
        )));
    }
    dst[..s.len()].copy_from_slice(s.as_bytes());
    Ok(())
}

/// Build `user_prg_header_t` (little-endian, packed) from a manifest for
/// `platform`; a manifest for another platform is refused, as the official
/// Librarian does.
pub fn build_header(
    platform: &Platform,
    manifest: &Value,
    payload_len: usize,
) -> Result<(Module, String, Vec<u8>), Error> {
    let h = &manifest["header"];
    let s = |k: &str| {
        h[k].as_str()
            .ok_or_else(|| bad(format!("manifest header.{k} missing")))
    };
    let u = |k: &str| {
        h[k].as_u64()
            .ok_or_else(|| bad(format!("manifest header.{k} missing")))
    };
    if s("platform")? != platform.name {
        return Err(bad(format!(
            "unit is for {:?}, not {}",
            s("platform")?,
            platform.name
        )));
    }
    let module = match s("module")? {
        "modfx" => Module::ModFx,
        "delfx" => Module::DelFx,
        "revfx" => Module::RevFx,
        "osc" => Module::Osc,
        m => return Err(bad(format!("unknown module {m:?}"))),
    };
    let name = s("name")?.to_string();
    let params = h["params"].as_array().cloned().unwrap_or_default();
    if params.len() > MAX_PARAMS {
        return Err(bad(format!(
            "{} params; at most {MAX_PARAMS}",
            params.len()
        )));
    }
    let mut b = vec![0u8; HEADER_LEN];
    b[0..2].copy_from_slice(&((module as u16) | u16::from(platform.id) << 8).to_le_bytes());
    b[2..6].copy_from_slice(&version(s("api")?)?.to_le_bytes());
    b[6..10].copy_from_slice(&(u("dev_id")? as u32).to_le_bytes());
    b[10..14].copy_from_slice(&(u("prg_id")? as u32).to_le_bytes());
    b[14..18].copy_from_slice(&version(s("version")?)?.to_le_bytes());
    put_str(&mut b[18..18 + NAME_LEN], &name, NAME_LEN)?;
    b[32..36].copy_from_slice(&(params.len() as u32).to_le_bytes());
    for (i, p) in params.iter().enumerate() {
        let o = 36 + i * 16;
        let (pname, min, max, ty) = (p[0].as_str(), p[1].as_i64(), p[2].as_i64(), p[3].as_str());
        let (Some(pname), Some(min), Some(max), Some(ty)) = (pname, min, max, ty) else {
            return Err(bad(format!("param {i} must be [name, min, max, type]")));
        };
        b[o] = min as i8 as u8;
        b[o + 1] = max as i8 as u8;
        // userprg.h: 0 percent, 1 bipolar percent, 2 select (typeless).
        b[o + 2] = match ty {
            "%" if min < 0 => 1,
            "%" => 0,
            _ => 2,
        };
        put_str(&mut b[o + 3..o + 3 + PARAM_NAME_LEN], pname, PARAM_NAME_LEN)?;
    }
    let load = (payload_len + SIG_LEN) as u32;
    b[HEADER_LEN - 4..].copy_from_slice(&load.to_le_bytes());
    Ok((module, name, b))
}

impl Unit {
    /// Read a unit file for `platform`.
    pub fn read(platform: &Platform, r: impl Read + Seek) -> Result<Unit, Error> {
        let mut zip = ZipArchive::new(r).map_err(|e| bad(e.to_string()))?;
        let mut manifest = None;
        let mut payload = None;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| bad(e.to_string()))?;
            let name = f.name().map_err(|e| bad(e.to_string()))?.into_owned();
            let mut data = Vec::new();
            f.read_to_end(&mut data).map_err(|e| bad(e.to_string()))?;
            if name.ends_with("/manifest.json") {
                manifest = Some(data);
            } else if name.ends_with("/payload.bin") {
                payload = Some(data);
            }
        }
        let manifest: Value =
            serde_json::from_slice(&manifest.ok_or_else(|| bad("no manifest.json"))?)
                .map_err(|e| bad(format!("manifest.json: {e}")))?;
        let payload = payload.ok_or_else(|| bad("no payload.bin"))?;
        let (module, name, header) = build_header(platform, &manifest, payload.len())?;
        let magic: &[u8] = match module {
            Module::Osc => b"UOSC",
            Module::ModFx => b"UMOD",
            Module::DelFx => b"UDEL",
            Module::RevFx => b"UREV",
        };
        if !payload.starts_with(magic) {
            return Err(bad("payload magic does not match the manifest's module"));
        }
        Ok(Unit {
            module,
            name,
            header,
            payload,
        })
    }

    /// The blob the synth stores: header, payload, zeroed signature area.
    pub fn blob(&self) -> Vec<u8> {
        blob(&self.header, &self.payload)
    }

    /// TABLE 7 slot data for upload; see [`slot_data`].
    pub fn slot_data(&self) -> Vec<u8> {
        slot_data(&self.blob())
    }
}

/// The blob the synth stores: header, payload, zeroed signature area.
pub fn blob(header: &[u8], payload: &[u8]) -> Vec<u8> {
    [header, payload, &[0; SIG_LEN]].concat()
}

/// TABLE 7 slot data for upload: `u32 size, u32 checksum, blob`. The
/// checksum is sent as 0, as the official Librarian does; the algorithm
/// the synth reports on read-back is not known yet.
pub fn slot_data(blob: &[u8]) -> Vec<u8> {
    [
        &(blob.len() as u32).to_le_bytes()[..],
        &0u32.to_le_bytes(),
        blob,
    ]
    .concat()
}

/// A unit's name from its TABLE 6 slot record (bytes 18..32, NUL-padded).
pub fn unit_name(header: &[u8]) -> String {
    let raw = header.get(18..32).unwrap_or_default();
    String::from_utf8_lossy(raw)
        .trim_end_matches('\0')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PROLOGUE: Platform = Platform {
        name: "prologue",
        id: 1,
    };

    #[test]
    fn platform_id_goes_into_the_target_word() {
        let m = json!({"header": {
            "platform": "prologue", "module": "modfx", "api": "1.1-0",
            "dev_id": 0, "prg_id": 0, "version": "1.0-0", "name": "chorus", "params": []
        }});
        let (module, _, h) = build_header(&PROLOGUE, &m, 4).unwrap();
        assert_eq!(module, Module::ModFx);
        assert_eq!(&h[0..2], &[0x01, 0x01]);
        let mut xd = m.clone();
        xd["header"]["platform"] = json!("minilogue-xd");
        assert_eq!(
            build_header(&PROLOGUE, &xd, 4).map(|_| ()),
            Err(Error::NotALibrary(
                "unit is for \"minilogue-xd\", not prologue".into()
            ))
        );
    }
}
