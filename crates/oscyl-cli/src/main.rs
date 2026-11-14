//! `oscyl`: command-line tool for the synths Oscyl supports.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use oscyl_driver::{Driver, LibProgram, LibraryData, SessionEvent, SessionOp};
use oscyl_midi::{DRIVERS, Device};
// User units, global and tuning data are minilogue xd matters the
// contract does not model; they run through `Device::run_units` on
// every logue synth (minilogue xd, prologue).
use oscyl_logue::unit::Unit;
use oscyl_midi::logue::{Module, UnitEvent as Event, UnitOp as Op, platform, unit_name};

#[derive(Parser)]
#[command(version, about = "Fast, open tooling for hardware synths")]
struct Cli {
    /// Only consider MIDI ports whose name contains this text.
    #[arg(long, global = true)]
    port: Option<String>,
    /// Which kind of synth, by driver id (see `oscyl ports`), when no file
    /// says.
    #[arg(long, global = true)]
    synth: Option<String>,
    /// Use a simulated synth preloaded with its demo programs instead of
    /// MIDI hardware.
    #[arg(long, global = true, env = "OSCYL_SIMULATOR",
          value_parser = clap::builder::BoolishValueParser::new())]
    simulator: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List MIDI ports and the synths Oscyl supports.
    Ports,
    /// Show the connected synth's port, MIDI channel and firmware.
    Info,
    /// Print the edit buffer (current, possibly unsaved, program).
    Current {
        /// Print the raw program bytes as hex instead of the parameter list.
        #[arg(long)]
        raw: bool,
    },
    /// Save every program (plus, on a minilogue xd, global, tuning and unit
    /// data) to a library: `.oscyllib` or one of the synth's own formats.
    Backup { out: PathBuf },
    /// List programs that differ between a library and the synth.
    Diff { library: PathBuf },
    /// Write a library to the synth. Only changed programs are sent.
    Restore {
        library: PathBuf,
        /// Write every program, even unchanged ones.
        #[arg(long)]
        all: bool,
    },
    /// Play a program from a file without storing it.
    Audition {
        file: PathBuf,
        /// Which program of a multi-program library (0-based).
        #[arg(long, default_value_t = 0)]
        index: usize,
    },
    /// Manage logue SDK user oscillators and effects (minilogue xd).
    Units {
        #[command(subcommand)]
        cmd: UnitsCmd,
    },
}

#[derive(Subcommand)]
enum UnitsCmd {
    /// Show what is loaded in every user slot.
    List,
    /// Load a .mnlgxdunit file into a user slot (1-based).
    Upload {
        file: PathBuf,
        #[arg(long)]
        slot: u8,
        /// Replace a unit already in that slot.
        #[arg(long)]
        force: bool,
    },
    /// Empty a user slot (1-based). The unit is backed up first.
    Clear {
        #[arg(long, value_enum)]
        module: ModuleArg,
        #[arg(long)]
        slot: u8,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum ModuleArg {
    Osc,
    Modfx,
    Delfx,
    Revfx,
}

impl From<ModuleArg> for Module {
    fn from(m: ModuleArg) -> Module {
        match m {
            ModuleArg::Osc => Module::Osc,
            ModuleArg::Modfx => Module::ModFx,
            ModuleArg::Delfx => Module::DelFx,
            ModuleArg::Revfx => Module::RevFx,
        }
    }
}

/// A command with its files read and checked, so a bad input fails before
/// the synth is touched.
enum Job {
    Info,
    Current {
        raw: bool,
    },
    Backup {
        out: PathBuf,
    },
    Diff {
        wanted: Vec<Vec<u8>>,
    },
    Restore {
        library: PathBuf,
        wanted: Vec<Vec<u8>>,
        all: bool,
    },
    Audition {
        program: Vec<u8>,
    },
    ListUnits,
    Upload {
        unit: Unit,
        slot: u8,
        force: bool,
    },
    Clear {
        module: Module,
        slot: u8,
    },
}

/// The job, and the driver its file belongs to.
/// `chosen`: the synth `--synth` names; unit commands default to the
/// first registered synth with units.
fn prepare(
    cmd: Cmd,
    chosen: Option<&'static dyn Driver>,
) -> Result<(Job, Option<&'static dyn Driver>)> {
    let logue = chosen.unwrap_or_else(|| {
        DRIVERS
            .iter()
            .copied()
            .find(|d| !d.capabilities().units.is_empty())
            .unwrap_or(DRIVERS[0])
    });
    Ok(match cmd {
        Cmd::Ports => unreachable!("handled before connecting"),
        Cmd::Info => (Job::Info, None),
        Cmd::Current { raw } => (Job::Current { raw }, None),
        Cmd::Backup { out } => {
            // Checked now: an unknown format must not cost a full read.
            let d = if ext(&out) == oscyl_core::native::EXT {
                None
            } else {
                Some(file_driver(&out)?)
            };
            (Job::Backup { out }, d)
        }
        Cmd::Diff { library } => {
            let (d, wanted) = programs(&library)?;
            (Job::Diff { wanted }, Some(d))
        }
        Cmd::Restore { library, all } => {
            let (d, wanted) = programs(&library)?;
            (
                Job::Restore {
                    library,
                    wanted,
                    all,
                },
                Some(d),
            )
        }
        Cmd::Audition { file, index } => {
            let (d, progs) = programs(&file)?;
            let n = progs.len();
            let program = progs
                .into_iter()
                .nth(index)
                .with_context(|| format!("{} has {n} programs", file.display()))?;
            (Job::Audition { program }, Some(d))
        }
        Cmd::Units { cmd } => match cmd {
            UnitsCmd::List => (Job::ListUnits, Some(logue)),
            UnitsCmd::Upload { file, slot, force } => {
                let f =
                    File::open(&file).with_context(|| format!("cannot open {}", file.display()))?;
                let p = platform(logue)
                    .with_context(|| format!("the {} has no user units", logue.info().model))?;
                let unit = Unit::read(&p, BufReader::new(f))
                    .with_context(|| format!("cannot read {}", file.display()))?;
                let slot = slot_index(unit.module, slot)?;
                (Job::Upload { unit, slot, force }, Some(logue))
            }
            UnitsCmd::Clear { module, slot } => {
                let module = Module::from(module);
                (
                    Job::Clear {
                        module,
                        slot: slot_index(module, slot)?,
                    },
                    Some(logue),
                )
            }
        },
    })
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Cmd::Ports = cli.cmd {
        for p in oscyl_midi::list_ports()? {
            println!("{p}");
        }
        for d in DRIVERS {
            let i = d.info();
            println!("synth: {} ({} {})", i.id, i.vendor, i.model);
        }
        return Ok(());
    }
    let chosen = match &cli.synth {
        Some(id) => Some(
            oscyl_midi::driver(id)
                .with_context(|| format!("no synth called {id:?}; see `oscyl ports`"))?,
        ),
        None => None,
    };
    let (job, from_file) = prepare(cli.cmd, chosen)?;
    if let (Some(a), Some(b)) = (chosen, from_file)
        && a.info().id != b.info().id
    {
        bail!(
            "that file is for the {}, not the {}",
            b.info().model,
            a.info().model
        );
    }
    let driver = chosen.or(from_file);
    let mut device = if cli.simulator {
        Device::demo(driver.unwrap_or(DRIVERS[0]))?
    } else {
        open(driver, cli.port.as_deref())?
    };
    run(job, &mut device)
}

/// The one connected synth of `driver` (any, when `None`) on a port
/// containing `port`.
fn open(driver: Option<&'static dyn Driver>, port: Option<&str>) -> Result<Device> {
    let found: Vec<_> = oscyl_midi::scan()?
        .into_iter()
        .filter(|c| driver.is_none_or(|d| d.info().id == c.driver.info().id))
        .filter(|c| port.is_none_or(|p| c.port.contains(p)))
        .collect();
    match found.as_slice() {
        [c] => Ok(Device::open(c)?),
        [] => bail!(
            "{}; connect it over USB and check `oscyl ports`",
            oscyl_midi::Error::NotFound
        ),
        _ => bail!(
            "more than one synth found ({}); pass --synth or a more specific --port",
            found
                .iter()
                .map(|c| c.port.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// The synth's slots in order: the library's programs go to these.
fn slots(d: &dyn Driver) -> Vec<u16> {
    d.capabilities()
        .banks
        .iter()
        .flat_map(|b| b.first..b.first + b.len)
        .collect()
}

/// The slots a library may write, from the first.
fn writable(d: &dyn Driver) -> Vec<u16> {
    d.capabilities()
        .banks
        .iter()
        .filter(|b| b.writable)
        .flat_map(|b| b.first..b.first + b.len)
        .collect()
}

fn xd_only(device: &Device) -> Result<()> {
    if device.driver().capabilities().units.is_empty() {
        bail!("the {} has no user units", device.driver().info().model);
    }
    Ok(())
}

fn run(job: Job, device: &mut Device) -> Result<()> {
    let d = device.driver();
    match job {
        Job::Info => {
            println!(
                "synth: {} {}\nport: {}\nchannel: {}",
                d.info().vendor,
                d.info().model,
                device.port_name(),
                device.channel() + 1,
            );
            if let Some(f) = device.firmware() {
                println!("firmware: {f}");
            }
        }
        Job::Current { raw } => {
            let mut cur = None;
            device.run(SessionOp::ReadCurrent, |e| {
                if let SessionEvent::Current(p) = e {
                    cur = Some(p.clone())
                }
            })?;
            print_program(d, &cur.context("no program received")?, raw)?;
        }
        Job::Backup { out } => {
            let lib = read_device(device, true)?.1;
            write_library(d, &lib, &out)?;
            eprintln!("saved {}", out.display());
        }
        Job::Diff { wanted } => {
            let on_device = read_device(device, false)?.0;
            let changed = changed(&on_device, &wanted);
            for &i in &changed {
                println!("{:03}  {}", i, d.name(&wanted[i])?);
            }
            eprintln!("{} of {} programs differ", changed.len(), wanted.len());
        }
        Job::Restore {
            library,
            wanted,
            all,
        } => {
            let (on_device, snapshot) = read_device(device, true)?;
            let backup = auto_backup_path(d)?;
            write_library(d, &snapshot, &backup)
                .context("automatic backup failed; nothing was written")?;
            eprintln!("backup of the synth saved to {}", backup.display());
            let mut todo: Vec<usize> = if all {
                (0..wanted.len()).collect()
            } else {
                changed(&on_device, &wanted)
            };
            // Factory banks are never written, even when a full backup
            // holding them is restored.
            let at = writable(d);
            let read_only = todo.iter().filter(|&&i| i >= at.len()).count();
            if read_only > 0 {
                eprintln!("{read_only} programs for read-only (factory) slots are left alone");
                todo.retain(|&i| i < at.len());
            }
            if todo.is_empty() {
                eprintln!("synth already matches {}", library.display());
                return Ok(());
            }
            let t0 = Instant::now();
            let batch = todo.iter().map(|&i| (at[i], wanted[i].clone())).collect();
            device.run(SessionOp::WritePrograms(batch), progress("writing"))?;
            eprintln!(
                "\nwrote and saved {} programs in {:.1?}",
                todo.len(),
                t0.elapsed()
            );
        }
        Job::Audition { program } => {
            let t0 = Instant::now();
            let name = d.name(&program)?;
            device.run(SessionOp::Audition(program), |_| {})?;
            eprintln!("auditioning \"{name}\" ({:.0?})", t0.elapsed());
        }
        Job::ListUnits => {
            xd_only(device)?;
            device.run_units(Op::ReadSlots, |e| {
                if let Event::Slot {
                    module,
                    slot,
                    header,
                } = e
                {
                    let name = header
                        .as_deref()
                        .map(unit_name)
                        .unwrap_or_else(|| "-".into());
                    println!("{:<6} {:>2}  {}", module.name(), slot + 1, name);
                }
            })?;
        }
        Job::Upload { unit, slot, force } => {
            xd_only(device)?;
            let m = unit.module;
            if let Some(existing) = slot_name(device, m, slot)? {
                if !force {
                    bail!(
                        "{} slot {} holds \"{existing}\"; pass --force to replace it",
                        m.name(),
                        slot + 1
                    );
                }
                backup_slot(device, m, slot)?;
            }
            let t0 = Instant::now();
            device.run_units(
                Op::UploadUnit {
                    module: m,
                    slot,
                    data: unit.slot_data(),
                },
                |_| {},
            )?;
            eprintln!(
                "loaded \"{}\" into {} slot {} ({:.1?})",
                unit.name,
                m.name(),
                slot + 1,
                t0.elapsed()
            );
        }
        Job::Clear { module, slot } => {
            xd_only(device)?;
            if slot_name(device, module, slot)?.is_none() {
                eprintln!("{} slot {} is already empty", module.name(), slot + 1);
                return Ok(());
            }
            backup_slot(device, module, slot)?;
            device.run_units(Op::ClearSlot { module, slot }, |_| {})?;
            eprintln!("cleared {} slot {}", module.name(), slot + 1);
        }
    }
    Ok(())
}

/// Library positions whose program differs from the synth's (or whose
/// slot was not read).
fn changed(on_device: &[Option<Vec<u8>>], wanted: &[Vec<u8>]) -> Vec<usize> {
    (0..wanted.len())
        .filter(|&i| on_device.get(i).and_then(Option::as_ref) != Some(&wanted[i]))
        .collect()
}

/// 1-based slot number from the user to a 0-based index.
fn slot_index(module: Module, slot: u8) -> Result<u8> {
    if slot == 0 || slot > module.slots() {
        bail!("{} slots are 1-{}", module.name(), module.slots());
    }
    Ok(slot - 1)
}

/// Name of the unit in a slot, or `None` when empty.
fn slot_name(device: &mut Device, module: Module, slot: u8) -> Result<Option<String>> {
    let mut name = None;
    device.run_units(Op::ReadSlots, |e| {
        if let Event::Slot {
            module: m,
            slot: s,
            header: Some(h),
        } = e
            && (*m, *s) == (module, slot)
        {
            name = Some(unit_name(h));
        }
    })?;
    Ok(name)
}

/// Save a slot's raw data under the backup directory before replacing it.
fn backup_slot(device: &mut Device, module: Module, slot: u8) -> Result<()> {
    let mut data = None;
    device.run_units(Op::ReadSlotData { module, slot }, |e| {
        if let Event::SlotData { data: d, .. } = e {
            data = Some(d.clone())
        }
    })?;
    let secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let name = format!("unit-{}-{:02}-{secs}.bin", module.name(), slot + 1);
    let path = data_dir()?.join("backups").join(name);
    std::fs::create_dir_all(path.parent().expect("has parent"))?;
    std::fs::write(&path, data.context("no slot data received")?)?;
    eprintln!("slot backed up to {}", path.display());
    Ok(())
}

fn print_program(d: &dyn Driver, p: &[u8], raw: bool) -> Result<()> {
    if raw {
        for (i, row) in p.chunks(16).enumerate() {
            let hex: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
            println!("{:04}: {}", i * 16, hex.join(" "));
        }
        return Ok(());
    }
    println!("name: {}", d.name(p)?);
    for (i, f) in d.fields().iter().enumerate() {
        println!("{} / {}: {}", f.group, f.name, d.get(p, i)?);
    }
    Ok(())
}

fn progress(label: &'static str) -> impl FnMut(&SessionEvent) {
    move |e| {
        if let SessionEvent::Progress { done, total } = e {
            eprint!("\r{label} {done}/{total}");
            let _ = std::io::stderr().flush();
        }
    }
}

/// Read all programs, and when `full` on a synth with user units (the
/// minilogue xd) its global, tuning and unit data too. Returns the
/// programs by position and a library snapshot.
fn read_device(device: &mut Device, full: bool) -> Result<(Vec<Option<Vec<u8>>>, LibraryData)> {
    let t0 = Instant::now();
    let all = slots(device.driver());
    let mut got: Vec<Option<Vec<u8>>> = vec![None; all.len()];
    let mut tick = progress("reading");
    device.run(SessionOp::ReadPrograms(all.clone()), |e| {
        tick(e);
        if let SessionEvent::Program(n, p) = e
            && let Some(i) = all.iter().position(|s| s == n)
        {
            got[i] = Some(p.clone());
        }
    })?;
    eprintln!(" programs ({:.1?})", t0.elapsed());
    let mut lib = LibraryData {
        programs: got
            .iter()
            .flatten()
            .map(|p| LibProgram {
                data: p.clone(),
                ..LibProgram::default()
            })
            .collect(),
        ..LibraryData::default()
    };
    if !full || device.driver().capabilities().units.is_empty() {
        return Ok((got, lib));
    }
    // Not part of the Librarian format; kept under oscyl/ so the Librarian
    // ignores them and Oscyl can restore them later.
    let mut extra = Vec::new();
    device.run_units(Op::ReadGlobal, |e| {
        if let Event::Global(g) = e {
            extra.push(("oscyl/global.bin".to_string(), g.clone()))
        }
    })?;
    device.run_units(Op::ReadTuning, |e| match e {
        Event::Scale(s, v) => extra.push((format!("oscyl/scale_{s}.bin"), v.clone())),
        Event::Octave(s, v) => extra.push((format!("oscyl/octave_{s}.bin"), v.clone())),
        _ => {}
    })?;
    let mut units = Vec::new();
    device.run_units(Op::ReadSlots, |e| {
        if let Event::Slot {
            module,
            slot,
            header: Some(_),
        } = e
        {
            units.push((*module, *slot))
        }
    })?;
    for (module, slot) in units {
        device.run_units(Op::ReadSlotData { module, slot }, |e| {
            if let Event::SlotData { data, .. } = e {
                extra.push((
                    format!("oscyl/units/{}_{:02}.bin", module.name(), slot),
                    data.clone(),
                ))
            }
        })?;
    }
    lib.extras.extend(extra);
    eprintln!("read everything in {:.1?}", t0.elapsed());
    Ok((got, lib))
}

fn ext(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// The driver whose vendor format `path` has.
fn file_driver(path: &Path) -> Result<&'static dyn Driver> {
    let e = ext(path);
    DRIVERS
        .iter()
        .copied()
        .find(|d| d.file_formats().iter().any(|f| f.ext == e))
        .with_context(|| format!("{}: no synth reads .{e} files", path.display()))
}

/// A library file's driver and contents.
fn read_library(path: &Path) -> Result<(&'static dyn Driver, LibraryData)> {
    let bytes = std::fs::read(path).with_context(|| format!("cannot open {}", path.display()))?;
    let cannot = || format!("cannot read {}", path.display());
    if ext(path) == oscyl_core::native::EXT {
        let (id, lib) =
            oscyl_core::native::read(std::io::Cursor::new(bytes)).with_context(cannot)?;
        let d = oscyl_midi::driver(&id)
            .with_context(|| format!("{}: made for an unknown synth ({id})", path.display()))?;
        return Ok((d, lib));
    }
    let d = file_driver(path)?;
    Ok((d, d.import(&ext(path), &bytes).with_context(cannot)?))
}

/// A library's programs, checked against what the synth can hold.
fn programs(path: &Path) -> Result<(&'static dyn Driver, Vec<Vec<u8>>)> {
    let (d, lib) = read_library(path)?;
    let room = slots(d).len();
    if lib.programs.len() > room {
        bail!(
            "library has {} programs; the {} holds {room}",
            lib.programs.len(),
            d.info().model
        );
    }
    Ok((d, lib.programs.into_iter().map(|p| p.data).collect()))
}

fn write_library(d: &dyn Driver, lib: &LibraryData, path: &Path) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut w = BufWriter::new(
        File::create(path).with_context(|| format!("cannot create {}", path.display()))?,
    );
    let e = ext(path);
    if e == oscyl_core::native::EXT {
        oscyl_core::native::write(&mut w, d.info().id, lib)?;
    } else {
        w.write_all(&d.export(&e, lib)?)?;
    }
    w.flush()?;
    Ok(())
}

fn data_dir() -> Result<PathBuf> {
    let base = match std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        Some(d) => PathBuf::from(d),
        None => {
            PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(".local/share")
        }
    };
    Ok(base.join("oscyl"))
}

/// In the synth's own format, so its vendor tools can open it too.
fn auto_backup_path(d: &dyn Driver) -> Result<PathBuf> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let ext = d
        .file_formats()
        .first()
        .map_or(oscyl_core::native::EXT, |f| f.ext);
    Ok(data_dir()?
        .join("backups")
        .join(format!("backup-{secs}.{ext}")))
}
