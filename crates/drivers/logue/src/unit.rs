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

/// How a unit parameter's value reads, and how a program stores it
/// (`userprg.h`'s `k_user_prg_param_type_*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// 0 to 100 %.
    Percent,
    /// -100 to +100 %.
    Bipolar,
    /// A plain number ("typeless"): a choice. The synth shows it one
    /// higher (0-9 reads 1-10).
    Select,
}

/// One of a unit's up to six parameters, as its header declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitParam {
    pub name: String,
    pub min: i8,
    pub max: i8,
    pub kind: ParamKind,
}

impl UnitParam {
    /// The values a program stores for this parameter, lowest and highest.
    /// A program stores what the unit is handed (`userosc.h`, on
    /// `OSC_PARAM`'s value: "0-200 for bipolar percent parameters. 0% at
    /// 100, -100% at 0. 0-100 for unipolar percent and typeless
    /// parameters"; the minilogue xd's MIDI implementation, note P24, gives
    /// the program's bytes the same three ranges): the value itself, or
    /// the value plus 100 when bipolar.
    pub fn stored(&self) -> (u8, u8) {
        let at = |v: i8| match self.kind {
            ParamKind::Bipolar => (i16::from(v) + 100).clamp(0, 200) as u8,
            ParamKind::Percent | ParamKind::Select => v.clamp(0, 100) as u8,
        };
        let (lo, hi) = (at(self.min), at(self.max));
        (lo.min(hi), hi.max(lo))
    }
}

/// What a unit says about itself: the inverse of [`build_header`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitInfo {
    pub module: Module,
    /// The SDK platform id ([`Platform::id`]).
    pub platform: u8,
    pub name: String,
    /// The developer's id, the unit's own and its version (`M << 16 | m <<
    /// 8 | p`): together with the name, which unit this is.
    pub dev_id: u32,
    pub prg_id: u32,
    pub version: u32,
    /// Its parameters, when the bytes read held them: a whole header does,
    /// a slot's status record (its first 32 bytes) does not.
    pub params: Option<Vec<UnitParam>>,
}

impl UnitInfo {
    /// From a unit's header (`user_prg_header_t`, [`HEADER_LEN`] bytes: the
    /// start of a slot's blob), or from a slot's status record, which is
    /// the header's first 32 bytes and so names no parameters.
    pub fn from_header(h: &[u8]) -> Result<UnitInfo, Error> {
        if h.len() < 32 {
            return Err(bad("a unit header is at least 32 bytes"));
        }
        let u32_at = |o: usize| u32::from_le_bytes([h[o], h[o + 1], h[o + 2], h[o + 3]]);
        let text = |raw: &[u8]| {
            let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
            String::from_utf8_lossy(&raw[..end]).into_owned()
        };
        let params = (h.len() >= 36 + MAX_PARAMS * 16).then(|| {
            let n = (u32_at(32) as usize).min(MAX_PARAMS);
            (0..n)
                .map(|i| {
                    let o = 36 + i * 16;
                    UnitParam {
                        name: text(&h[o + 3..o + 3 + PARAM_NAME_LEN]),
                        min: h[o] as i8,
                        max: h[o + 1] as i8,
                        kind: match h[o + 2] {
                            0 => ParamKind::Percent,
                            1 => ParamKind::Bipolar,
                            _ => ParamKind::Select,
                        },
                    }
                })
                .collect()
        });
        Ok(UnitInfo {
            module: Module::from_id(h[0])?,
            platform: h[1],
            name: text(&h[18..18 + NAME_LEN]),
            dev_id: u32_at(6),
            prg_id: u32_at(10),
            version: u32_at(14),
            params,
        })
    }

    /// Whether `other` is this unit (a slot's status against a file's
    /// header, say): the same section, platform, name, ids and version.
    pub fn same_unit(&self, other: &UnitInfo) -> bool {
        (self.module, self.platform, &self.name) == (other.module, other.platform, &other.name)
            && (self.dev_id, self.prg_id, self.version)
                == (other.dev_id, other.prg_id, other.version)
    }
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

    /// What the unit says about itself, parameters included.
    pub fn info(&self) -> UnitInfo {
        UnitInfo::from_header(&self.header).expect("a header this module built")
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

    #[test]
    fn a_header_reads_back_as_its_manifest() {
        let m = json!({"header": {
            "platform": "prologue", "module": "osc", "api": "1.1-0",
            "dev_id": 0x4B4F5247u32, "prg_id": 7, "version": "1.2-3", "name": "waves",
            "params": [
                ["Wave A", 0, 45, ""],
                ["Sub Mix", 0, 100, "%"],
                ["Tilt", -100, 100, "%"],
                ["Twelve chars", 2, 9, ""]
            ]
        }});
        let (_, _, h) = build_header(&PROLOGUE, &m, 64).unwrap();
        let info = UnitInfo::from_header(&h).unwrap();
        assert_eq!(
            (info.module, info.platform, &*info.name),
            (Module::Osc, 1, "waves")
        );
        assert_eq!(
            (info.dev_id, info.prg_id, info.version),
            (0x4B4F_5247, 7, 0x01_02_03)
        );
        let p = |name: &str, min, max, kind| UnitParam {
            name: name.into(),
            min,
            max,
            kind,
        };
        let params = info.params.clone().unwrap();
        assert_eq!(
            params,
            [
                p("Wave A", 0, 45, ParamKind::Select),
                p("Sub Mix", 0, 100, ParamKind::Percent),
                p("Tilt", -100, 100, ParamKind::Bipolar),
                p("Twelve chars", 2, 9, ParamKind::Select),
            ]
        );
        // What a program stores: the value, or the value plus 100 when bipolar.
        let stored: Vec<_> = params.iter().map(UnitParam::stored).collect();
        assert_eq!(stored, [(0, 45), (0, 100), (0, 200), (2, 9)]);
        // A slot's status record is the header's start: the same unit, no parameters.
        let status = UnitInfo::from_header(&h[..32]).unwrap();
        assert_eq!(status.params, None);
        assert!(status.same_unit(&info));
        let unit = Unit {
            module: Module::Osc,
            name: "waves".into(),
            header: h,
            payload: b"UOSC".to_vec(),
        };
        assert_eq!(unit.info(), info);
        assert!(UnitInfo::from_header(&[0; 31]).is_err());
        assert!(UnitInfo::from_header(&[9; 64]).is_err(), "no module 9");
    }
}
