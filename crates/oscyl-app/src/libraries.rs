//! Oscyl's own libraries: one `<id>.oscyllib` per library in a folder,
//! with `index.json` holding their order, names and times. GTK-free, so
//! tests drive it in a temporary folder.
//!
//! Libraries used to be stored as `<id>.mnlgxdlib`; [`Store::open`]
//! converts those once, keeping the originals in `before-oscyllib/`.

use std::fs::File;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use oscyl_core::native;
use oscyl_driver::{Driver, LibraryData};
use serde::{Deserialize, Serialize};

use crate::i18n::tr;
use crate::state::Doc;

/// The index. Before the native format it was `index.json`, which older
/// builds still rewrite, so the native store keeps its own.
const INDEX: &str = "libraries.json";
/// The index before the native format; older builds still read it.
const LEGACY_INDEX: &str = "index.json";
const EXT: &str = oscyl_core::native::EXT;
/// How libraries were stored before the native format.
const LEGACY_EXT: &str = "mnlgxdlib";
/// Where migration keeps the original files and index.
pub const BACKUP: &str = "before-oscyllib";
/// The driver of every library made before libraries recorded one: the
/// minilogue xd, the only synth Oscyl knew then.
const LEGACY_DRIVER: &str = "korg-minilogue-xd";
const TRASH: &str = "trash";

/// One library in the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    /// A random 64-bit hex string; also the file's stem.
    pub id: String,
    /// The display name; names may repeat.
    pub name: String,
    /// Seconds since the Unix epoch.
    pub created: u64,
    pub changed: u64,
    /// The driver whose programs the library holds.
    #[serde(default = "xd_driver")]
    pub driver: String,
}

fn xd_driver() -> String {
    LEGACY_DRIVER.to_string()
}

/// A place in the libraries tree: a library (by id) or a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Node {
    Library(String),
    Folder(Folder),
}

impl Node {
    pub fn id(&self) -> &str {
        match self {
            Node::Library(id) => id,
            Node::Folder(f) => &f.id,
        }
    }

    /// Whether node `id` is this one or inside it.
    pub fn contains(&self, id: &str) -> bool {
        let mut all = Vec::new();
        self.walk(&mut all);
        all.iter().any(|n| n.id() == id)
    }

    /// This node and everything inside it, depth first.
    fn walk<'a>(&'a self, out: &mut Vec<&'a Node>) {
        out.push(self);
        if let Node::Folder(f) = self {
            for c in &f.children {
                c.walk(out);
            }
        }
    }
}

/// A folder of libraries and folders, in order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folder {
    /// A random 64-bit hex string, like a library's.
    pub id: String,
    pub name: String,
    /// Shows its contents in the sidebar.
    #[serde(default = "open_by_default")]
    pub open: bool,
    #[serde(default)]
    pub children: Vec<Node>,
}

fn open_by_default() -> bool {
    true
}

/// One line of the tree as the sidebar shows it, depth first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub depth: usize,
    pub kind: LineKind,
    /// Every folder around it is open.
    pub shown: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    Library(String),
    Folder {
        id: String,
        name: String,
        open: bool,
    },
}

/// `index.json`: the libraries' details and the tree they sit in. An
/// index from before folders has no tree: its libraries go at the top
/// level, in their order.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    libraries: Vec<Meta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tree: Option<Vec<Node>>,
}

/// Something deleted (a library, or a folder with everything inside), its
/// files in `trash/` until restored or purged.
#[derive(Debug, Clone)]
pub struct Trashed {
    /// The folder it was in (`None`: the top level) and where in it.
    pub parent: Option<String>,
    pub at: usize,
    pub node: Node,
    /// The libraries inside, with their details.
    pub metas: Vec<Meta>,
}

/// The libraries folder.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    /// The libraries' details, in the tree's depth-first order.
    entries: Vec<Meta>,
    tree: Vec<Node>,
}

/// The children of folder `id` (`None`: the top level) in `tree`.
fn children_mut<'a>(tree: &'a mut Vec<Node>, id: Option<&str>) -> Option<&'a mut Vec<Node>> {
    let Some(id) = id else { return Some(tree) };
    for n in tree.iter_mut() {
        if let Node::Folder(f) = n {
            if f.id == id {
                return Some(&mut f.children);
            }
            if let Some(c) = children_mut(&mut f.children, Some(id)) {
                return Some(c);
            }
        }
    }
    None
}

/// Where node `id` is: its folder (`None`: the top level) and index.
fn find(tree: &[Node], id: &str, parent: Option<&str>) -> Option<(Option<String>, usize)> {
    for (i, n) in tree.iter().enumerate() {
        if n.id() == id {
            return Some((parent.map(str::to_string), i));
        }
        if let Node::Folder(f) = n
            && let Some(found) = find(&f.children, id, Some(&f.id))
        {
            return Some(found);
        }
    }
    None
}

/// Keep only libraries in `known` (once each) and folders seen once,
/// recording every id kept in `seen`.
fn sanitise(
    nodes: Vec<Node>,
    known: &std::collections::HashSet<String>,
    seen: &mut std::collections::HashSet<String>,
) -> Vec<Node> {
    nodes
        .into_iter()
        .filter_map(|n| match n {
            Node::Library(id) => {
                (known.contains(&id) && seen.insert(id.clone())).then_some(Node::Library(id))
            }
            Node::Folder(mut f) => {
                if !seen.insert(f.id.clone()) {
                    return None;
                }
                f.children = sanitise(std::mem::take(&mut f.children), known, seen);
                Some(Node::Folder(f))
            }
        })
        .collect()
}

/// The first of `name(1)`, `name(2)`… that is not `taken`.
fn free_name(name: impl Fn(usize) -> String, taken: impl Fn(&str) -> bool) -> String {
    (1..)
        .map(name)
        .find(|name| !taken(name))
        .expect("a free name")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Write `path` atomically: a temporary file in the same folder, then a
/// rename over the target, so a failed or interrupted write leaves the old
/// file as it was.
pub fn write_atomic(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> Result<(), String>,
) -> Result<(), String> {
    let name = path
        .file_name()
        .ok_or_else(|| format!("not a file: {}", path.display()))?;
    let tmp = path.with_file_name(format!(".{}.part", name.to_string_lossy()));
    let result = (|| {
        let mut w = BufWriter::new(File::create(&tmp).map_err(text)?);
        write(&mut w)?;
        let f = w.into_inner().map_err(text)?;
        f.sync_all().map_err(text)?;
        std::fs::rename(&tmp, path).map_err(text)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn write_native(path: &Path, driver: &str, data: &LibraryData) -> Result<(), String> {
    write_atomic(path, |w| native::write(w, driver, data).map_err(text))
}

fn read_native(path: &Path) -> Result<(String, LibraryData), String> {
    let f = File::open(path).map_err(text)?;
    native::read(BufReader::new(f)).map_err(text)
}

/// Convert every `<id>.mnlgxdlib` in `dir` to `<id>.oscyllib`, then move
/// the original into `before-oscyllib/` (with a copy of the old index). A
/// converted file is read back and compared before its original moves. An
/// original that cannot be read moves unconverted; one whose new file could
/// not be written stays put, to be retried next time. Originals are never
/// deleted or overwritten; a rerun after an interruption finishes the job.
fn migrate(dir: &Path) -> Result<(), String> {
    let mut legacy: Vec<(PathBuf, String)> = std::fs::read_dir(dir)
        .map_err(text)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == LEGACY_EXT))
        .filter_map(|p| {
            let stem = p.file_stem()?.to_str()?.to_string();
            (!stem.starts_with('.')).then_some((p, stem))
        })
        .collect();
    if legacy.is_empty() {
        return Ok(());
    }
    legacy.sort();
    let backup = dir.join(BACKUP);
    std::fs::create_dir_all(&backup).map_err(text)?;
    if !backup.join(LEGACY_INDEX).exists()
        && let Ok(old) = std::fs::read(dir.join(LEGACY_INDEX))
    {
        write_atomic(&backup.join(LEGACY_INDEX), |w| {
            w.write_all(&old).map_err(text)
        })?;
    }
    for (path, stem) in legacy {
        let native = dir.join(format!("{stem}.{EXT}"));
        if !native.exists() {
            let legacy = oscyl_midi::driver(LEGACY_DRIVER).expect("the xd is registered");
            let data = match std::fs::read(&path)
                .map_err(text)
                .and_then(|b| legacy.import(LEGACY_EXT, &b).map_err(text))
            {
                Ok(data) => data,
                Err(e) => {
                    eprintln!(
                        "oscyl: keeping unreadable {} in {BACKUP}/: {e}",
                        path.display()
                    );
                    keep(&path, &backup);
                    continue;
                }
            };
            let written = write_native(&native, LEGACY_DRIVER, &data).and_then(|()| {
                match read_native(&native)? {
                    (_, back) if back == data => Ok(()),
                    _ => Err("converted library does not read back equal".into()),
                }
            });
            if let Err(e) = written {
                let _ = std::fs::remove_file(&native);
                eprintln!(
                    "oscyl: could not convert {}, will retry: {e}",
                    path.display()
                );
                continue;
            }
        }
        keep(&path, &backup);
    }
    Ok(())
}

/// Move `path` into `backup` under a name nothing there has yet.
fn keep(path: &Path, backup: &Path) {
    let name = path
        .file_name()
        .expect("listed as a file")
        .to_string_lossy();
    let (stem, ext) = name.rsplit_once('.').unwrap_or((&name, ""));
    let free = std::iter::once(backup.join(&*name))
        .chain((2..).map(|n| backup.join(format!("{stem}.{n}.{ext}"))))
        .find(|p| !p.exists())
        .expect("an unused name");
    if let Err(e) = std::fs::rename(path, &free) {
        eprintln!("oscyl: could not move {} to {BACKUP}/: {e}", path.display());
    }
}

/// A fresh random id. The std hasher's keys are seeded from the OS for
/// each process; the time and a counter keep ids in one process apart.
fn random_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u64(COUNT.fetch_add(1, Ordering::Relaxed));
    h.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    );
    format!("{:016x}", h.finish())
}

impl Store {
    /// Open (creating it if needed) the folder `dir`: empty the trash left
    /// by the last run and read the index, rebuilding it from the library
    /// files present when it is missing, damaged or out of step (a rebuilt
    /// index has every library at the top level).
    pub fn open(dir: PathBuf) -> Result<Store, String> {
        std::fs::create_dir_all(&dir).map_err(text)?;
        migrate(&dir)?;
        let _ = std::fs::remove_dir_all(dir.join(TRASH));
        let index = |name: &str| std::fs::read(dir.join(name)).ok();
        let (bytes, legacy_index) = match index(INDEX) {
            Some(b) => (Some(b), false),
            None => (index(LEGACY_INDEX), true),
        };
        let read: Option<Index> = bytes.and_then(|b| serde_json::from_slice(&b).ok());
        let mut changed = legacy_index || read.as_ref().is_none_or(|i| i.tree.is_none());
        let Index { libraries, tree } = read.unwrap_or_default();
        let mut entries: Vec<Meta> = Vec::new();
        let mut known = std::collections::HashSet::new();
        for m in libraries {
            // A library whose migration must be retried keeps its entry.
            let present = dir.join(format!("{}.{EXT}", m.id)).is_file()
                || dir.join(format!("{}.{LEGACY_EXT}", m.id)).is_file();
            if present && known.insert(m.id.clone()) {
                entries.push(m);
            } else {
                changed = true;
            }
        }
        let tree = tree.unwrap_or_else(|| {
            entries
                .iter()
                .map(|m| Node::Library(m.id.clone()))
                .collect()
        });
        let mut found: Vec<(PathBuf, String)> = std::fs::read_dir(&dir)
            .map_err(text)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == EXT))
            .filter_map(|p| {
                let stem = p.file_stem()?.to_str()?.to_string();
                (!stem.starts_with('.')).then_some((p, stem))
            })
            .filter(|(_, stem)| !known.contains(stem))
            .collect();
        found.sort();
        for (path, stem) in found {
            let modified = std::fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or_else(now);
            known.insert(stem.clone());
            let driver = File::open(&path)
                .ok()
                .and_then(|f| native::read(BufReader::new(f)).ok())
                .map_or_else(xd_driver, |(d, _)| d);
            entries.push(Meta {
                id: stem.clone(),
                name: stem,
                created: modified,
                changed: modified,
                driver,
            });
            changed = true;
        }
        let before = serde_json::to_string(&tree).unwrap_or_default();
        let mut seen = std::collections::HashSet::new();
        let mut tree = sanitise(tree, &known, &mut seen);
        // Libraries the tree lost go at the top level, in their order.
        for m in &entries {
            if !seen.contains(&m.id) {
                tree.push(Node::Library(m.id.clone()));
            }
        }
        changed |= serde_json::to_string(&tree).unwrap_or_default() != before;
        let mut store = Store { dir, entries, tree };
        store.reorder();
        if changed {
            store.write_index()?;
        }
        Ok(store)
    }

    /// Put the entries in the tree's depth-first order.
    fn reorder(&mut self) {
        let ids = self.library_ids();
        let mut metas = std::mem::take(&mut self.entries);
        self.entries = ids
            .iter()
            .filter_map(|id| {
                let i = metas.iter().position(|m| m.id == *id)?;
                Some(metas.swap_remove(i))
            })
            .collect();
    }

    /// Every library's id, depth first.
    fn library_ids(&self) -> Vec<String> {
        let mut all = Vec::new();
        for n in &self.tree {
            n.walk(&mut all);
        }
        all.into_iter()
            .filter_map(|n| match n {
                Node::Library(id) => Some(id.clone()),
                Node::Folder(_) => None,
            })
            .collect()
    }

    #[cfg(test)]
    pub fn tree(&self) -> &[Node] {
        &self.tree
    }

    /// The tree as lines, depth first, with their depth and whether every
    /// folder around them is open.
    pub fn outline(&self) -> Vec<Line> {
        fn go(nodes: &[Node], depth: usize, shown: bool, out: &mut Vec<Line>) {
            for n in nodes {
                match n {
                    Node::Library(id) => out.push(Line {
                        depth,
                        kind: LineKind::Library(id.clone()),
                        shown,
                    }),
                    Node::Folder(f) => {
                        out.push(Line {
                            depth,
                            kind: LineKind::Folder {
                                id: f.id.clone(),
                                name: f.name.clone(),
                                open: f.open,
                            },
                            shown,
                        });
                        go(&f.children, depth + 1, shown && f.open, out);
                    }
                }
            }
        }
        let mut out = Vec::new();
        go(&self.tree, 0, true, &mut out);
        out
    }

    /// "Library 1", "Library 2"…: the first no library has.
    pub fn new_library_name(&self) -> String {
        free_name(
            |n| tr("Library {number}").replace("{number}", &n.to_string()),
            |name| self.entries.iter().any(|m| m.name == name),
        )
    }

    /// "Folder 1", "Folder 2"…: the first no folder has.
    pub fn new_folder_name(&self) -> String {
        let taken: Vec<String> = self
            .outline()
            .into_iter()
            .filter_map(|l| match l.kind {
                LineKind::Folder { name, .. } => Some(name),
                LineKind::Library(_) => None,
            })
            .collect();
        free_name(
            |n| tr("Folder {number}").replace("{number}", &n.to_string()),
            |name| taken.iter().any(|t| t == name),
        )
    }

    /// Where node `id` is: its folder (`None`: the top level) and index.
    pub fn locate(&self, id: &str) -> Option<(Option<String>, usize)> {
        find(&self.tree, id, None)
    }

    /// Where node `id` is among its folder's contents, and how many
    /// there are.
    pub fn siblings(&self, id: &str) -> Option<(usize, usize)> {
        let (parent, at) = self.locate(id)?;
        let len = match parent {
            Some(p) => self.folder(&p)?.children.len(),
            None => self.tree.len(),
        };
        Some((at, len))
    }

    fn node(&self, id: &str) -> Option<&Node> {
        let mut all = Vec::new();
        for n in &self.tree {
            n.walk(&mut all);
        }
        all.into_iter().find(|n| n.id() == id)
    }

    fn folder(&self, id: &str) -> Option<&Folder> {
        match self.node(id)? {
            Node::Folder(f) => Some(f),
            Node::Library(_) => None,
        }
    }

    fn folder_mut(&mut self, id: &str) -> Option<&mut Folder> {
        let (parent, at) = self.locate(id)?;
        match children_mut(&mut self.tree, parent.as_deref())?.get_mut(at)? {
            Node::Folder(f) => Some(f),
            Node::Library(_) => None,
        }
    }

    /// Change the tree with `f`, write the index, and undo the change if
    /// that fails.
    fn change<R>(&mut self, f: impl FnOnce(&mut Store) -> Result<R, String>) -> Result<R, String> {
        let (tree, entries) = (self.tree.clone(), self.entries.clone());
        let r = f(self).and_then(|r| {
            self.reorder();
            self.write_index().map(|()| r)
        });
        if r.is_err() {
            self.tree = tree;
            self.entries = entries;
        }
        r
    }

    /// Insert `node` in folder `parent` (`None`: the top level) at `at`,
    /// or last; last at the top level when that folder has gone.
    fn put(&mut self, parent: Option<&str>, at: usize, node: Node) {
        let (parent, at) = match parent {
            Some(p) if self.folder(p).is_none() => (None, usize::MAX),
            p => (p, at),
        };
        let list = children_mut(&mut self.tree, parent).expect("the folder is there");
        list.insert(at.min(list.len()), node);
    }

    fn take(&mut self, id: &str) -> Result<(Option<String>, usize, Node), String> {
        let (parent, at) = self
            .locate(id)
            .ok_or_else(|| format!("nothing called {id}"))?;
        let list = children_mut(&mut self.tree, parent.as_deref()).expect("found it there");
        let node = list.remove(at);
        Ok((parent, at, node))
    }

    pub fn entries(&self) -> &[Meta] {
        &self.entries
    }

    pub fn path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.{EXT}"))
    }

    fn pos(&self, id: &str) -> Result<usize, String> {
        self.entries
            .iter()
            .position(|m| m.id == id)
            .ok_or_else(|| format!("no library {id}"))
    }

    fn write_index(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&Index {
            libraries: self.entries.clone(),
            tree: Some(self.tree.clone()),
        })
        .map_err(text)?;
        write_atomic(&self.dir.join(INDEX), |w| w.write_all(&json).map_err(text))
    }

    /// Library `id`'s synth and contents. A library of a synth this build
    /// has no driver for is an error naming it.
    pub fn read(&self, id: &str) -> Result<(&'static dyn Driver, LibraryData), String> {
        let (driver, data) = read_native(&self.path(id))?;
        let d = oscyl_midi::driver(&driver)
            .ok_or_else(|| format!("this library is for a synth Oscyl does not know ({driver})"))?;
        Ok((d, data))
    }

    fn new_id(&self) -> String {
        loop {
            let id = random_id();
            if self.locate(&id).is_none() && !self.path(&id).exists() {
                return id;
            }
        }
    }

    /// Add a library named `name` holding `lib`, last in folder `parent`
    /// (`None`: the top level); returns its id.
    pub fn create(
        &mut self,
        name: &str,
        driver: &dyn Driver,
        lib: &LibraryData,
        parent: Option<&str>,
    ) -> Result<String, String> {
        let id = self.new_id();
        write_native(&self.path(&id), driver.info().id, lib)?;
        let t = now();
        let made = self.change(|s| {
            s.entries.push(Meta {
                id: id.clone(),
                name: name.to_string(),
                created: t,
                changed: t,
                driver: driver.info().id.to_string(),
            });
            s.put(parent, usize::MAX, Node::Library(id.clone()));
            Ok(())
        });
        if let Err(e) = made {
            let _ = std::fs::remove_file(self.path(&id));
            return Err(e);
        }
        Ok(id)
    }

    /// Add an empty folder named `name`, last in folder `parent`; returns
    /// its id.
    pub fn new_folder(&mut self, name: &str, parent: Option<&str>) -> Result<String, String> {
        let id = self.new_id();
        self.change(|s| {
            s.put(
                parent,
                usize::MAX,
                Node::Folder(Folder {
                    id: id.clone(),
                    name: name.to_string(),
                    open: true,
                    children: Vec::new(),
                }),
            );
            Ok(id.clone())
        })
    }

    /// Write library `id`'s programs.
    /// Write library `id`, for the synth its index entry names.
    pub fn save(&mut self, id: &str, lib: &LibraryData) -> Result<(), String> {
        let i = self.pos(id)?;
        write_native(&self.path(id), &self.entries[i].driver, lib)?;
        self.entries[i].changed = now();
        self.write_index()
    }

    /// Rename a library or a folder.
    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        self.change(|s| {
            if let Ok(i) = s.pos(id) {
                s.entries[i].name = name.to_string();
            } else {
                s.folder_mut(id)
                    .ok_or_else(|| format!("nothing called {id}"))?
                    .name = name.to_string();
            }
            Ok(())
        })
    }

    /// Open or close folder `id` (remembered for the next run).
    pub fn set_open(&mut self, id: &str, open: bool) -> Result<(), String> {
        self.change(|s| {
            s.folder_mut(id)
                .ok_or_else(|| format!("no folder {id}"))?
                .open = open;
            Ok(())
        })
    }

    /// Copy library `id` as `name`, right after it; returns the copy's id.
    pub fn duplicate(&mut self, id: &str, name: &str) -> Result<String, String> {
        let (parent, at) = self.locate(id).ok_or_else(|| format!("no library {id}"))?;
        let new = self.new_id();
        let bytes = std::fs::read(self.path(id)).map_err(text)?;
        write_atomic(&self.path(&new), |w| w.write_all(&bytes).map_err(text))?;
        let t = now();
        let driver = self.entries[self.pos(id)?].driver.clone();
        let made = self.change(|s| {
            s.entries.push(Meta {
                id: new.clone(),
                name: name.to_string(),
                created: t,
                changed: t,
                driver,
            });
            s.put(parent.as_deref(), at + 1, Node::Library(new.clone()));
            Ok(())
        });
        if let Err(e) = made {
            let _ = std::fs::remove_file(self.path(&new));
            return Err(e);
        }
        Ok(new)
    }

    fn trashed(&self, id: &str) -> PathBuf {
        self.dir.join(TRASH).join(format!("{id}.{EXT}"))
    }

    /// Move a library, or a folder with everything inside, to the trash;
    /// [`Store::restore`] brings it back.
    pub fn delete(&mut self, id: &str) -> Result<Trashed, String> {
        std::fs::create_dir_all(self.dir.join(TRASH)).map_err(text)?;
        let (tree, entries) = (self.tree.clone(), self.entries.clone());
        let (parent, at, node) = self.take(id)?;
        let mut inside = Vec::new();
        node.walk(&mut inside);
        let ids: Vec<String> = inside
            .iter()
            .filter_map(|n| match n {
                Node::Library(id) => Some(id.clone()),
                Node::Folder(_) => None,
            })
            .collect();
        let mut moved = Vec::new();
        let mut result = Ok(());
        for lib in &ids {
            match std::fs::rename(self.path(lib), self.trashed(lib)) {
                Ok(()) => moved.push(lib.clone()),
                Err(e) => {
                    result = Err(text(e));
                    break;
                }
            }
        }
        let metas: Vec<Meta> = self
            .entries
            .iter()
            .filter(|m| ids.contains(&m.id))
            .cloned()
            .collect();
        self.entries.retain(|m| !ids.contains(&m.id));
        if let Err(e) = result.and_then(|()| self.write_index()) {
            for lib in moved {
                let _ = std::fs::rename(self.trashed(&lib), self.path(&lib));
            }
            self.tree = tree;
            self.entries = entries;
            return Err(e);
        }
        Ok(Trashed {
            parent,
            at,
            node,
            metas,
        })
    }

    /// Bring back something deleted where it was (at the top level when
    /// its folder is gone).
    pub fn restore(&mut self, t: Trashed) -> Result<(), String> {
        for m in &t.metas {
            std::fs::rename(self.trashed(&m.id), self.path(&m.id)).map_err(text)?;
        }
        self.change(|s| {
            s.entries.extend(t.metas.iter().cloned());
            s.put(t.parent.as_deref(), t.at, t.node.clone());
            Ok(())
        })
    }

    /// Remove deleted libraries' files for good.
    pub fn purge(&self, t: &Trashed) {
        for m in &t.metas {
            let _ = std::fs::remove_file(self.trashed(&m.id));
        }
    }

    /// Empty the trash (when the app quits).
    pub fn purge_all(&self) {
        let _ = std::fs::remove_dir_all(self.dir.join(TRASH));
    }

    /// Put node `id` in folder `parent` (`None`: the top level) before
    /// what is now at `at` there (past the end: last). A folder can't go
    /// into itself or a folder inside it.
    pub fn place(&mut self, id: &str, parent: Option<&str>, at: usize) -> Result<(), String> {
        let node = self
            .node(id)
            .ok_or_else(|| format!("nothing called {id}"))?;
        if let Some(p) = parent {
            if node.contains(p) {
                return Err("a folder can't go inside itself".into());
            }
            if self.folder(p).is_none() {
                return Err(format!("no folder {p}"));
            }
        }
        self.change(|s| {
            let (from_parent, from, node) = s.take(id)?;
            // Taking it out moved the later ones in the same folder up.
            let at = if from_parent.as_deref() == parent && from < at {
                at - 1
            } else {
                at
            };
            s.put(parent, at, node);
            Ok(())
        })
    }

    /// Move node `id` one place up (-1) or down (1) in its folder; false
    /// when it is already first or last there.
    pub fn step(&mut self, id: &str, by: isize) -> Result<bool, String> {
        let (parent, at) = self
            .locate(id)
            .ok_or_else(|| format!("nothing called {id}"))?;
        let len = children_mut(&mut self.tree, parent.as_deref()).map_or(0, |c| c.len());
        let Some(to) = at.checked_add_signed(by).filter(|&to| to < len) else {
            return Ok(false);
        };
        self.change(|s| {
            children_mut(&mut s.tree, parent.as_deref())
                .expect("found it there")
                .swap(at, to);
            Ok(true)
        })
    }
}

/// A library in the sidebar: one of Oscyl's own, or a file opened from
/// elsewhere.
#[derive(Debug)]
pub struct Lib {
    pub doc: Doc,
    /// Its id in the [`Store`]; `None` for a file opened from elsewhere.
    pub id: Option<String>,
    /// The document revision last written.
    pub saved: u64,
    /// The last autosave failed: the library has unsaved changes.
    pub failed: bool,
    /// Why the file could not be read; the library is then left alone.
    pub broken: Option<String>,
}

impl Lib {
    pub fn file(doc: Doc) -> Lib {
        Lib {
            doc,
            id: None,
            saved: 0,
            failed: false,
            broken: None,
        }
    }

    pub fn managed(doc: Doc, id: String) -> Lib {
        Lib {
            saved: doc.revision(),
            doc,
            id: Some(id),
            failed: false,
            broken: None,
        }
    }

    pub fn is_managed(&self) -> bool {
        self.id.is_some()
    }

    /// Has changes the person must still save (a file library's edits, or
    /// an autosave that failed).
    pub fn unsaved(&self) -> bool {
        if self.is_managed() {
            self.failed
        } else {
            self.doc.is_dirty()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oscyl_driver::LibProgram;

    fn xd() -> &'static dyn Driver {
        oscyl_midi::driver(LEGACY_DRIVER).unwrap()
    }

    /// A fresh folder under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let dir = std::env::temp_dir().join(format!(
                "oscyl-test-{name}-{}-{}",
                std::process::id(),
                random_id()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn lib(names: &[&str]) -> LibraryData {
        LibraryData {
            programs: names
                .iter()
                .map(|n| {
                    let mut data = xd().init_program();
                    xd().set_name(&mut data, n).unwrap();
                    LibProgram {
                        data,
                        ..LibProgram::default()
                    }
                })
                .collect(),
            ..LibraryData::default()
        }
    }

    /// The names of library `id`'s programs.
    fn read(s: &Store, id: &str) -> Vec<String> {
        let (d, data) = s.read(id).unwrap();
        data.programs
            .iter()
            .map(|p| d.name(&p.data).unwrap())
            .collect()
    }

    fn named(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    fn names(store: &Store) -> Vec<&str> {
        store.entries().iter().map(|m| m.name.as_str()).collect()
    }

    /// The tree as text: "A [F: B [G: C]] D", a closed folder's name
    /// after a "-".
    fn shape(store: &Store) -> String {
        fn go(store: &Store, nodes: &[Node]) -> String {
            nodes
                .iter()
                .map(|n| match n {
                    Node::Library(id) => store
                        .entries()
                        .iter()
                        .find(|m| m.id == *id)
                        .map_or("?".into(), |m| m.name.clone()),
                    Node::Folder(f) => format!(
                        "[{}{}: {}]",
                        if f.open { "" } else { "-" },
                        f.name,
                        go(store, &f.children)
                    ),
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        go(store, store.tree())
    }

    #[test]
    fn create_save_and_read_round_trip() {
        let t = TempDir::new("round");
        let mut s = Store::open(t.0.clone()).unwrap();
        let id = s
            .create("Gig set", xd(), &lib(&["One", "Two"]), None)
            .unwrap();
        assert_eq!(id.len(), 16);
        assert_eq!(read(&s, &id), named(&["One", "Two"]));
        s.save(&id, &lib(&["Three"])).unwrap();
        let again = Store::open(t.0.clone()).unwrap();
        assert_eq!(names(&again), ["Gig set"]);
        assert_eq!(read(&again, &id), ["Three"]);
    }

    #[test]
    fn new_names_skip_taken_ones() {
        let t = TempDir::new("names");
        let mut s = Store::open(t.0.clone()).unwrap();
        assert_eq!(s.new_library_name(), "Library 1");
        s.create("Library 1", xd(), &lib(&[]), None).unwrap();
        assert_eq!(s.new_library_name(), "Library 2");
        let f = s.new_folder("Folder 1", None).unwrap();
        s.new_folder("Folder 2", Some(&f)).unwrap();
        assert_eq!(s.new_folder_name(), "Folder 3");
    }

    #[test]
    fn rename_duplicate_and_reorder_persist() {
        let t = TempDir::new("order");
        let mut s = Store::open(t.0.clone()).unwrap();
        let a = s.create("A", xd(), &lib(&["a"]), None).unwrap();
        let b = s.create("B", xd(), &lib(&["b"]), None).unwrap();
        s.rename(&a, "Studio").unwrap();
        let c = s.duplicate(&a, "Studio copy").unwrap();
        assert_ne!(c, a);
        assert_eq!(names(&s), ["Studio", "Studio copy", "B"]);
        assert_eq!(s.read(&c).unwrap().1, s.read(&a).unwrap().1);
        s.place(&b, None, 0).unwrap();
        assert_eq!(
            names(&Store::open(t.0.clone()).unwrap()),
            ["B", "Studio", "Studio copy"]
        );
        assert!(s.step(&b, 1).unwrap());
        assert!(!s.step(&c, 1).unwrap(), "already last");
        assert_eq!(names(&s), ["Studio", "B", "Studio copy"]);
    }

    #[test]
    fn folders_nest_and_things_move_into_before_and_after() {
        let t = TempDir::new("tree");
        let mut s = Store::open(t.0.clone()).unwrap();
        let a = s.create("A", xd(), &lib(&["a"]), None).unwrap();
        let f = s.new_folder("F", None).unwrap();
        s.create("B", xd(), &lib(&["b"]), Some(&f)).unwrap();
        let g = s.new_folder("G", Some(&f)).unwrap();
        let c = s.create("C", xd(), &lib(&["c"]), Some(&g)).unwrap();
        let d = s.create("D", xd(), &lib(&["d"]), None).unwrap();
        assert_eq!(shape(&s), "A [F: B [G: C]] D");
        assert_eq!(names(&s), ["A", "B", "C", "D"], "depth-first order");
        // Into a folder (at its end), then before and after a row at its
        // level.
        s.place(&a, Some(&g), usize::MAX).unwrap();
        assert_eq!(shape(&s), "[F: B [G: C A]] D");
        s.place(&d, Some(&f), 0).unwrap();
        assert_eq!(shape(&s), "[F: D B [G: C A]]");
        s.place(&a, Some(&f), 2).unwrap();
        assert_eq!(shape(&s), "[F: D B A [G: C]]", "before G");
        s.place(&d, Some(&f), 3).unwrap();
        assert_eq!(shape(&s), "[F: B A D [G: C]]", "after A, same folder");
        s.place(&g, None, usize::MAX).unwrap();
        assert_eq!(shape(&s), "[F: B A D] [G: C]");
        assert_eq!(s.locate(&c), Some((Some(g.clone()), 0)));
        assert_eq!(names(&s), ["B", "A", "D", "C"]);
        // Everything persists, folders included.
        assert_eq!(
            shape(&Store::open(t.0.clone()).unwrap()),
            "[F: B A D] [G: C]"
        );
    }

    #[test]
    fn a_folder_cannot_go_into_itself_or_a_folder_inside_it() {
        let t = TempDir::new("cycle");
        let mut s = Store::open(t.0.clone()).unwrap();
        let f = s.new_folder("F", None).unwrap();
        let g = s.new_folder("G", Some(&f)).unwrap();
        let h = s.new_folder("H", Some(&g)).unwrap();
        assert!(s.place(&f, Some(&f), 0).is_err());
        assert!(s.place(&f, Some(&h), 0).is_err());
        assert!(s.place(&g, Some(&h), 0).is_err());
        assert_eq!(shape(&s), "[F: [G: [H: ]]]", "left as it was");
        s.place(&h, None, 0).unwrap();
        s.place(&f, Some(&h), 0).unwrap();
        assert_eq!(shape(&s), "[H: [F: [G: ]]]");
    }

    #[test]
    fn deleting_a_folder_takes_everything_inside_and_restore_brings_it_back() {
        let t = TempDir::new("subtree");
        let mut s = Store::open(t.0.clone()).unwrap();
        s.create("A", xd(), &lib(&["a"]), None).unwrap();
        let f = s.new_folder("F", None).unwrap();
        let b = s.create("B", xd(), &lib(&["b"]), Some(&f)).unwrap();
        let g = s.new_folder("G", Some(&f)).unwrap();
        let c = s.create("C", xd(), &lib(&["c"]), Some(&g)).unwrap();
        s.create("D", xd(), &lib(&["d"]), None).unwrap();
        let gone = s.delete(&f).unwrap();
        assert_eq!(shape(&s), "A D");
        assert!(!s.path(&b).exists() && !s.path(&c).exists());
        assert_eq!(gone.metas.len(), 2);
        s.restore(gone).unwrap();
        assert_eq!(shape(&s), "A [F: B [G: C]] D");
        assert_eq!(read(&s, &c), named(&["c"]));
        // A library deleted from a folder goes back into it; when the
        // folder has gone too, to the top level.
        let one = s.delete(&c).unwrap();
        s.restore(one).unwrap();
        assert_eq!(shape(&s), "A [F: B [G: C]] D");
        let one = s.delete(&c).unwrap();
        let folder = s.delete(&g).unwrap();
        s.purge(&folder);
        s.restore(one).unwrap();
        assert_eq!(shape(&s), "A [F: B] D C");
    }

    #[test]
    fn open_folders_are_remembered() {
        let t = TempDir::new("open");
        let mut s = Store::open(t.0.clone()).unwrap();
        let f = s.new_folder("F", None).unwrap();
        s.create("A", xd(), &lib(&["a"]), Some(&f)).unwrap();
        s.set_open(&f, false).unwrap();
        s.rename(&f, "Live").unwrap();
        let again = Store::open(t.0.clone()).unwrap();
        assert_eq!(shape(&again), "[-Live: A]");
        let lines = again.outline();
        assert_eq!(lines.len(), 2);
        assert_eq!((lines[1].depth, lines[1].shown), (1, false));
    }

    #[test]
    fn an_index_from_before_folders_puts_libraries_at_the_top_level() {
        let t = TempDir::new("migrate");
        for id in ["00000000000000aa", "00000000000000bb"] {
            legacy(&t.0, id, &lib(&["a"]));
        }
        std::fs::write(
            t.0.join(LEGACY_INDEX),
            br#"{"libraries": [
                {"id": "00000000000000bb", "name": "Second", "created": 1, "changed": 2},
                {"id": "00000000000000aa", "name": "First", "created": 1, "changed": 2}
            ]}"#,
        )
        .unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        assert_eq!(shape(&s), "Second First", "same order, top level");
        let json = std::fs::read_to_string(t.0.join(INDEX)).unwrap();
        assert!(json.contains("\"tree\""), "written back with its tree");
    }

    /// Write a `.mnlgxdlib` as the app stored libraries before the native
    /// format.
    fn legacy(dir: &Path, id: &str, l: &LibraryData) -> Vec<u8> {
        let bytes = xd().export(LEGACY_EXT, l).unwrap();
        std::fs::write(dir.join(format!("{id}.{LEGACY_EXT}")), &bytes).unwrap();
        bytes
    }

    const OLD_INDEX: &[u8] = br#"{"libraries": [
        {"id": "00000000000000aa", "name": "Live", "created": 1, "changed": 2},
        {"id": "00000000000000bb", "name": "Studio", "created": 3, "changed": 4}
    ], "tree": [
        {"folder": {"id": "00000000000000ff", "name": "Sets", "open": false,
                    "children": [{"library": "00000000000000aa"}]}},
        {"library": "00000000000000bb"}
    ]}"#;

    #[test]
    fn migration_keeps_names_folders_and_order() {
        let t = TempDir::new("migrate-native");
        let a = legacy(&t.0, "00000000000000aa", &lib(&["Lead", "Bass"]));
        let b = legacy(&t.0, "00000000000000bb", &lib(&["Pad"]));
        std::fs::write(t.0.join(LEGACY_INDEX), OLD_INDEX).unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        assert_eq!(shape(&s), "[-Sets: Live] Studio");
        assert_eq!(read(&s, "00000000000000aa"), named(&["Lead", "Bass"]));
        assert_eq!(read(&s, "00000000000000bb"), named(&["Pad"]));
        assert!(s.entries().iter().all(|m| m.driver == "korg-minilogue-xd"));
        let backup = t.0.join(BACKUP);
        for (id, bytes) in [("00000000000000aa", a), ("00000000000000bb", b)] {
            assert!(t.0.join(format!("{id}.{EXT}")).is_file());
            assert!(!t.0.join(format!("{id}.{LEGACY_EXT}")).exists());
            assert_eq!(
                std::fs::read(backup.join(format!("{id}.{LEGACY_EXT}"))).unwrap(),
                bytes
            );
        }
        assert_eq!(std::fs::read(backup.join(LEGACY_INDEX)).unwrap(), OLD_INDEX);
        assert_eq!(
            std::fs::read(t.0.join(LEGACY_INDEX)).unwrap(),
            OLD_INDEX,
            "the old index stays for older builds"
        );
        // A second run changes nothing.
        let again = Store::open(t.0.clone()).unwrap();
        assert_eq!(shape(&again), "[-Sets: Live] Studio");
    }

    #[test]
    fn unreadable_legacy_file_is_kept_in_backup_and_others_migrate() {
        let t = TempDir::new("migrate-bad");
        std::fs::write(t.0.join(format!("00000000000000aa.{LEGACY_EXT}")), b"").unwrap();
        legacy(&t.0, "00000000000000bb", &lib(&["Pad"]));
        std::fs::write(t.0.join(LEGACY_INDEX), OLD_INDEX).unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        assert_eq!(read(&s, "00000000000000bb"), named(&["Pad"]));
        let kept =
            t.0.join(BACKUP)
                .join(format!("00000000000000aa.{LEGACY_EXT}"));
        assert_eq!(std::fs::read(kept).unwrap(), b"");
        assert!(!t.0.join(format!("00000000000000aa.{EXT}")).exists());
    }

    #[test]
    fn rerun_after_interrupted_migration_finishes_it() {
        let t = TempDir::new("migrate-rerun");
        let original = legacy(&t.0, "00000000000000bb", &lib(&["Pad"]));
        Store::open(t.0.clone()).unwrap();
        let native = t.0.join(format!("00000000000000bb.{EXT}"));
        let converted = std::fs::read(&native).unwrap();
        // As if the run had stopped after writing the native file.
        legacy(&t.0, "00000000000000bb", &lib(&["Pad"]));
        let s = Store::open(t.0.clone()).unwrap();
        assert_eq!(std::fs::read(&native).unwrap(), converted);
        assert!(!t.0.join(format!("00000000000000bb.{LEGACY_EXT}")).exists());
        assert_eq!(
            std::fs::read(
                t.0.join(BACKUP)
                    .join(format!("00000000000000bb.{LEGACY_EXT}"))
            )
            .unwrap(),
            original
        );
        assert_eq!(read(&s, "00000000000000bb"), named(&["Pad"]));
    }

    #[test]
    fn failed_write_leaves_original_in_place_and_keeps_its_entry() {
        let t = TempDir::new("migrate-writefail");
        let original = legacy(&t.0, "00000000000000bb", &lib(&["Pad"]));
        std::fs::write(t.0.join(LEGACY_INDEX), OLD_INDEX).unwrap();
        // Block the temporary file the native write uses.
        let part = t.0.join(format!(".00000000000000bb.{EXT}.part"));
        std::fs::create_dir(&part).unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        let here = t.0.join(format!("00000000000000bb.{LEGACY_EXT}"));
        assert_eq!(
            std::fs::read(&here).unwrap(),
            original,
            "original stays put"
        );
        assert!(names(&s).contains(&"Studio"), "its entry survives");
        std::fs::remove_dir(&part).unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        assert!(names(&s).contains(&"Studio"));
        assert_eq!(read(&s, "00000000000000bb"), named(&["Pad"]));
        assert!(!here.exists(), "migrated on the next launch");
    }

    #[test]
    fn a_second_original_never_overwrites_the_backup() {
        let t = TempDir::new("migrate-twice");
        let first = legacy(&t.0, "00000000000000bb", &lib(&["Pad"]));
        Store::open(t.0.clone()).unwrap();
        let second = legacy(&t.0, "00000000000000bb", &lib(&["Other"]));
        Store::open(t.0.clone()).unwrap();
        let backup = t.0.join(BACKUP);
        let kept: Vec<Vec<u8>> = std::fs::read_dir(&backup)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == LEGACY_EXT))
            .map(|p| std::fs::read(p).unwrap())
            .collect();
        assert_eq!(kept.len(), 2);
        assert!(kept.contains(&first) && kept.contains(&second));
    }

    #[test]
    fn an_older_build_rewriting_the_old_index_loses_nothing() {
        let t = TempDir::new("migrate-downgrade");
        legacy(&t.0, "00000000000000aa", &lib(&["Lead"]));
        legacy(&t.0, "00000000000000bb", &lib(&["Pad"]));
        std::fs::write(t.0.join(LEGACY_INDEX), OLD_INDEX).unwrap();
        Store::open(t.0.clone()).unwrap();
        // What an older build writes when it finds no .mnlgxdlib files.
        std::fs::write(t.0.join(LEGACY_INDEX), br#"{"libraries":[],"tree":[]}"#).unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        assert_eq!(shape(&s), "[-Sets: Live] Studio");
    }

    #[test]
    fn read_names_an_unknown_driver() {
        let t = TempDir::new("other-synth");
        let mut s = Store::open(t.0.clone()).unwrap();
        let id = s.create("A", xd(), &lib(&["a"]), None).unwrap();
        write_atomic(&s.path(&id), |w| {
            native::write(w, "acme-unknown", &lib(&["a"])).map_err(text)
        })
        .unwrap();
        let e = s.read(&id).err().unwrap();
        assert!(e.contains("acme-unknown"), "{e}");
    }

    #[test]
    fn store_keeps_a_rev2_library() {
        let t = TempDir::new("rev2");
        let rev2 = oscyl_midi::driver("sequential-prophet-rev2").unwrap();
        let mut s = Store::open(t.0.clone()).unwrap();
        let mut data = LibraryData::default();
        let mut p = rev2.init_program();
        rev2.set_name(&mut p, "Wide Pad").unwrap();
        data.programs.push(LibProgram {
            data: p,
            author: "Me".into(),
            comment: "warm".into(),
        });
        let id = s.create("Rev2 set", rev2, &data, None).unwrap();
        let again = Store::open(t.0.clone()).unwrap();
        assert_eq!(again.entries()[0].driver, "sequential-prophet-rev2");
        let (d, back) = again.read(&id).unwrap();
        assert_eq!(d.info().id, "sequential-prophet-rev2");
        assert_eq!(back, data);
        s.save(&id, &LibraryData::default()).unwrap();
        assert_eq!(s.read(&id).unwrap().0.info().id, "sequential-prophet-rev2");
    }

    #[test]
    fn no_legacy_files_no_backup_folder() {
        let t = TempDir::new("migrate-none");
        let mut s = Store::open(t.0.clone()).unwrap();
        s.create("A", xd(), &lib(&["a"]), None).unwrap();
        Store::open(t.0.clone()).unwrap();
        assert!(!t.0.join(BACKUP).exists());
    }

    #[test]
    fn delete_and_restore() {
        let t = TempDir::new("trash");
        let mut s = Store::open(t.0.clone()).unwrap();
        s.create("A", xd(), &lib(&["a"]), None).unwrap();
        let b = s.create("B", xd(), &lib(&["b"]), None).unwrap();
        s.create("C", xd(), &lib(&["c"]), None).unwrap();
        let gone = s.delete(&b).unwrap();
        assert!(!s.path(&b).exists());
        assert_eq!(names(&s), ["A", "C"]);
        s.restore(gone).unwrap();
        assert_eq!(names(&s), ["A", "B", "C"]);
        assert_eq!(read(&s, &b), named(&["b"]));
        // Purged, it is gone for good; a new run empties the trash.
        let gone = s.delete(&b).unwrap();
        s.purge(&gone);
        assert!(s.restore(gone).is_err());
        let again = s.delete(&s.entries()[0].id.clone()).unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        assert!(!s.trashed(&again.metas[0].id).exists());
        assert_eq!(names(&s), ["C"]);
    }

    #[test]
    fn a_failed_write_leaves_the_old_file() {
        let t = TempDir::new("atomic");
        let mut s = Store::open(t.0.clone()).unwrap();
        let id = s.create("A", xd(), &lib(&["Kept"]), None).unwrap();
        let err = write_atomic(&s.path(&id), |w| {
            w.write_all(b"half a zip").map_err(text)?;
            Err("disk full".into())
        });
        assert_eq!(err, Err("disk full".to_string()));
        assert_eq!(read(&s, &id), named(&["Kept"]));
        let leftovers = std::fs::read_dir(&t.0)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".part"))
            .count();
        assert_eq!(leftovers, 0, "the temporary file is removed");
        // A library that cannot be written is not added.
        s.dir = t.0.join("missing");
        assert!(s.create("B", xd(), &lib(&["b"]), None).is_err());
        assert_eq!(s.entries().len(), 1);
    }

    #[test]
    fn a_lost_or_damaged_index_is_rebuilt_from_the_files() {
        let t = TempDir::new("index");
        let mut s = Store::open(t.0.clone()).unwrap();
        let a = s.create("A", xd(), &lib(&["a"]), None).unwrap();
        let b = s.create("B", xd(), &lib(&["b"]), None).unwrap();
        std::fs::write(t.0.join(INDEX), b"{ not json").unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        let mut ids: Vec<&str> = s.entries().iter().map(|m| m.id.as_str()).collect();
        ids.sort();
        let mut want = [a.as_str(), b.as_str()];
        want.sort();
        assert_eq!(ids, want);
        assert!(
            s.entries().iter().all(|m| m.name == m.id),
            "names from file stems"
        );
        // A file gone from the folder drops out; a new one joins at the end.
        std::fs::remove_file(s.path(&a)).unwrap();
        std::fs::write(t.0.join(format!("extra.{EXT}")), b"").unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        assert_eq!(names(&s), [b.as_str(), "extra"]);
    }

    #[test]
    fn a_rebuilt_index_has_every_library_at_the_top_level() {
        let t = TempDir::new("rebuild");
        let mut s = Store::open(t.0.clone()).unwrap();
        let f = s.new_folder("F", None).unwrap();
        let a = s.create("A", xd(), &lib(&["a"]), Some(&f)).unwrap();
        std::fs::remove_file(t.0.join(INDEX)).unwrap();
        let s = Store::open(t.0.clone()).unwrap();
        assert_eq!(shape(&s), a, "no folders, the library at the top");
    }
}
