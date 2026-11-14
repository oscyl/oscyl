//! The official Librarians' file formats: library, preset pack and single
//! program (xd `.mnlgxdlib`/`.mnlgxdpreset`/`.mnlgxdprog`, prologue
//! `.prlglib`/`.prlgpreset`/`.prlgprog`).
//!
//! All are ZIP archives with a `FileInformation.xml` index that lists each
//! entry. Programs are typed; every other entry (favorites, live sets,
//! tuning tables, preset information, unknown members) is kept verbatim so a
//! read followed by a write loses nothing. Models differ only in the names a
//! [`Product`] gives.

use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};
use std::marker::PhantomData;

use quick_xml::escape::{escape, unescape};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use oscyl_driver::{LibProgram, LibraryData};

use crate::{Error, Model};

pub const INDEX: &str = "FileInformation.xml";
pub const PROGRAM_DATA: &str = "ProgramData";

/// The names a model's Librarian writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Product {
    /// `<Product>` in the index: `minilogue xd`, `prologue`.
    pub name: &'static str,
    /// Root element of `.prog_info`: `xd_ProgramInformation`,
    /// `prologue_ProgramInformation`.
    pub info_root: &'static str,
    /// Entry kinds counted as `Num<kind>` attributes of `<Contents>`, in
    /// order.
    pub counts: &'static [&'static str],
}

/// Author metadata stored next to each program (`Prog_NNN.prog_info`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgInfo {
    pub programmer: String,
    pub comment: String,
}

/// One child of `<Contents>` in `FileInformation.xml`, e.g. `ProgramData`
/// with fields `Information` and `ProgramBinary` naming archive members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: String,
    pub fields: Vec<(String, String)>,
}

impl Entry {
    fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// A Librarian file of model `M`, held in memory.
pub struct Library<M> {
    /// Index entries in file order.
    pub entries: Vec<Entry>,
    /// Every archive member except `FileInformation.xml`.
    pub files: BTreeMap<String, Vec<u8>>,
    model: PhantomData<fn() -> M>,
}

impl<M> Default for Library<M> {
    fn default() -> Self {
        Library {
            entries: vec![],
            files: BTreeMap::new(),
            model: PhantomData,
        }
    }
}

impl<M> Clone for Library<M> {
    fn clone(&self) -> Self {
        Library {
            entries: self.entries.clone(),
            files: self.files.clone(),
            model: PhantomData,
        }
    }
}

impl<M> PartialEq for Library<M> {
    fn eq(&self, other: &Self) -> bool {
        (&self.entries, &self.files) == (&other.entries, &other.files)
    }
}

impl<M> Eq for Library<M> {}

impl<M> std::fmt::Debug for Library<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library")
            .field("entries", &self.entries)
            .field("files", &self.files)
            .finish()
    }
}

fn archive_err(e: impl std::fmt::Display) -> Error {
    Error::Archive(e.to_string())
}

fn xml_err(e: impl std::fmt::Display) -> Error {
    Error::NotALibrary(e.to_string())
}

impl<M: Model> Library<M> {
    pub fn read(r: impl Read + Seek) -> Result<Library<M>, M::Error> {
        let mut zip = ZipArchive::new(r).map_err(|e| Error::NotALibrary(e.to_string()))?;
        let mut files = BTreeMap::new();
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(archive_err)?;
            if f.is_dir() {
                continue;
            }
            let mut data = Vec::new();
            f.read_to_end(&mut data).map_err(archive_err)?;
            let name = f.name().map_err(archive_err)?.into_owned();
            files.insert(name, data);
        }
        let index = files
            .remove(INDEX)
            .ok_or_else(|| Error::NotALibrary(format!("missing {INDEX}")))?;
        let entries = parse_index(&M::PRODUCT, &index)?;
        let lib = Library::<M> {
            entries,
            files,
            model: PhantomData,
        };
        lib.programs()?; // validate program members up front
        Ok(lib)
    }

    pub fn write(&self, w: impl Write + Seek) -> Result<(), M::Error> {
        let mut zip = ZipWriter::new(w);
        // The same bytes on every host: without this the archive says which
        // system wrote it.
        let opts = SimpleFileOptions::default().system(zip::System::Unix);
        zip.start_file(INDEX, opts).map_err(archive_err)?;
        zip.write_all(self.index_xml().as_bytes())
            .map_err(archive_err)?;
        for (name, data) in &self.files {
            zip.start_file(name.as_str(), opts).map_err(archive_err)?;
            zip.write_all(data).map_err(archive_err)?;
        }
        zip.finish().map_err(archive_err)?;
        Ok(())
    }

    /// Programs in index order, with their author metadata.
    pub fn programs(&self) -> Result<Vec<(M::Program, ProgInfo)>, M::Error> {
        self.entries
            .iter()
            .filter(|e| e.kind == PROGRAM_DATA)
            .map(|e| {
                let bin = e
                    .field("ProgramBinary")
                    .and_then(|n| self.files.get(n))
                    .ok_or_else(|| Error::NotALibrary("program entry without binary".into()))?;
                let info = e
                    .field("Information")
                    .and_then(|n| self.files.get(n))
                    .map(|x| parse_prog_info(x))
                    .transpose()?
                    .unwrap_or_default();
                Ok((M::parse(bin)?, info))
            })
            .collect()
    }

    /// Append a program as `Prog_NNN`, numbered after the existing ones.
    pub fn push_program(&mut self, program: &M::Program, info: &ProgInfo) {
        let n = self
            .entries
            .iter()
            .filter(|e| e.kind == PROGRAM_DATA)
            .count();
        let (info_name, bin_name) = (
            format!("Prog_{n:03}.prog_info"),
            format!("Prog_{n:03}.prog_bin"),
        );
        self.files.insert(
            info_name.clone(),
            prog_info_xml(&M::PRODUCT, info).into_bytes(),
        );
        self.files
            .insert(bin_name.clone(), M::bytes(program).to_vec());
        self.entries.push(Entry {
            kind: PROGRAM_DATA.into(),
            fields: vec![
                ("Information".into(), info_name),
                ("ProgramBinary".into(), bin_name),
            ],
        });
    }

    /// `FileInformation.xml` for this library.
    pub fn index_xml(&self) -> String {
        let p = M::PRODUCT;
        let count = |k: &str| self.entries.iter().filter(|e| e.kind == k).count();
        let mut x =
            String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\n<KorgMSLibrarian_Data>\n");
        x += &format!("  <Product>{}</Product>\n", p.name);
        x += "  <Contents";
        for (i, k) in p.counts.iter().enumerate() {
            // The Librarians wrap the attribute list after every third.
            x += if i > 0 && i % 3 == 0 {
                "\n            "
            } else {
                " "
            };
            x += &format!("Num{k}=\"{}\"", count(k));
        }
        x += ">\n";
        for e in &self.entries {
            x += &format!("    <{}>\n", e.kind);
            for (k, v) in &e.fields {
                x += &format!("      <{k}>{}</{k}>\n", escape(v.as_str()));
            }
            x += &format!("    </{}>\n", e.kind);
        }
        x += "  </Contents>\n</KorgMSLibrarian_Data>\n";
        x
    }
}

/// The member holding the non-program index entries in [`LibraryData`]
/// extras.
const DATA_INDEX: &str = INDEX;

/// A Librarian file as neutral library data: programs with their author
/// metadata; every other entry and member kept in the extras, the
/// non-program index entries as `FileInformation.xml`.
pub fn to_data<M: Model>(lib: &Library<M>) -> Result<LibraryData, M::Error> {
    let programs = lib
        .programs()?
        .into_iter()
        .map(|(p, i)| LibProgram {
            data: M::bytes(&p).to_vec(),
            author: i.programmer,
            comment: i.comment,
        })
        .collect();
    let rest = Library::<M> {
        entries: lib
            .entries
            .iter()
            .filter(|e| e.kind != PROGRAM_DATA)
            .cloned()
            .collect(),
        ..Library::default()
    };
    let mut extras: BTreeMap<String, Vec<u8>> = lib
        .files
        .iter()
        .filter(|(n, _)| !n.starts_with("Prog_"))
        .map(|(n, d)| (n.clone(), d.clone()))
        .collect();
    extras.insert(DATA_INDEX.into(), rest.index_xml().into_bytes());
    Ok(LibraryData { programs, extras })
}

/// The Librarian file for neutral library data: programs first, then the
/// other entries and members from the extras.
pub fn from_data<M: Model>(data: &LibraryData) -> Result<Library<M>, M::Error> {
    let mut lib = Library::<M>::default();
    for p in &data.programs {
        let info = ProgInfo {
            programmer: p.author.clone(),
            comment: p.comment.clone(),
        };
        lib.push_program(&M::parse(&p.data)?, &info);
    }
    for (name, bytes) in &data.extras {
        if name == DATA_INDEX {
            lib.entries.extend(
                parse_index(&M::PRODUCT, bytes)?
                    .into_iter()
                    .filter(|e| e.kind != PROGRAM_DATA),
            );
        } else {
            lib.files.insert(name.clone(), bytes.clone());
        }
    }
    Ok(lib)
}

/// Walk an XML document and call `leaf(path, text)` for every element that
/// contains text, with entities resolved. `start(path)` sees every opening tag.
fn walk_xml(
    xml: &[u8],
    mut start: impl FnMut(&[String]),
    mut leaf: impl FnMut(&[String], String),
) -> Result<(), Error> {
    let mut r = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let mut path: Vec<String> = Vec::new();
    let mut raw = String::new();
    loop {
        match r.read_event_into(&mut buf).map_err(xml_err)? {
            Event::Start(e) => {
                path.push(e.name().as_ref().to_string());
                start(&path);
                raw.clear();
            }
            Event::Text(t) => raw.push_str(&t),
            Event::GeneralRef(g) => raw += &format!("&{};", &*g),
            Event::End(_) => {
                let text = unescape(raw.trim()).map_err(xml_err)?.into_owned();
                if !text.is_empty() {
                    leaf(&path, text);
                }
                raw.clear();
                path.pop();
            }
            Event::Eof => return Ok(()),
            _ => {}
        }
        buf.clear();
    }
}

/// Parse `<Contents>` children: each is an element whose children are
/// simple `<Name>text</Name>` fields.
fn parse_index(product: &Product, xml: &[u8]) -> Result<Vec<Entry>, Error> {
    let entries = std::cell::RefCell::new(Vec::<Entry>::new());
    let mut found = None;
    walk_xml(
        xml,
        |path| {
            if let [_, c, kind] = path
                && c == "Contents"
            {
                entries.borrow_mut().push(Entry {
                    kind: kind.clone(),
                    fields: Vec::new(),
                });
            }
        },
        |path, text| match path {
            [_, p] if p == "Product" => found = Some(text),
            [_, c, _, field] if c == "Contents" => {
                if let Some(e) = entries.borrow_mut().last_mut() {
                    e.fields.push((field.clone(), text));
                }
            }
            _ => {}
        },
    )?;
    match found.as_deref() {
        Some(p) if p == product.name => Ok(entries.into_inner()),
        other => Err(Error::NotALibrary(format!("product is {other:?}"))),
    }
}

fn parse_prog_info(xml: &[u8]) -> Result<ProgInfo, Error> {
    let mut info = ProgInfo::default();
    walk_xml(
        xml,
        |_| {},
        |path, text| match path.last().map(String::as_str) {
            Some("Programmer") => info.programmer = text,
            Some("Comment") => info.comment = text,
            _ => {}
        },
    )?;
    Ok(info)
}

fn prog_info_xml(p: &Product, i: &ProgInfo) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\n<{root}>\n  <Programmer>{}</Programmer>\n  <Comment>{}</Comment>\n</{root}>\n",
        escape(i.programmer.as_str()),
        escape(i.comment.as_str()),
        root = p.info_root,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{Test, program};
    use std::io::Cursor;

    #[test]
    fn product_names_the_index_and_info() {
        let mut lib = Library::<Test>::default();
        let info = ProgInfo {
            programmer: "Ann".into(),
            comment: "c".into(),
        };
        lib.push_program(&program(1), &info);
        let x = lib.index_xml();
        assert!(x.contains("<Product>prologue</Product>"));
        assert!(x.contains(
            "<Contents NumLivesetData=\"0\" NumProgramData=\"1\" NumPresetInformation=\"0\">"
        ));
        let info_xml = String::from_utf8(lib.files["Prog_000.prog_info"].clone()).unwrap();
        assert!(info_xml.contains("<prologue_ProgramInformation>"));
        let mut out = Cursor::new(vec![]);
        lib.write(&mut out).unwrap();
        let back = Library::<Test>::read(Cursor::new(out.into_inner())).unwrap();
        assert_eq!(back.programs().unwrap(), vec![(program(1), info)]);
    }

    #[test]
    fn another_products_file_is_refused() {
        let x = "<KorgMSLibrarian_Data><Product>minilogue xd</Product><Contents/></KorgMSLibrarian_Data>";
        assert!(matches!(
            parse_index(&Test::PRODUCT, x.as_bytes()),
            Err(Error::NotALibrary(_))
        ));
    }
}
