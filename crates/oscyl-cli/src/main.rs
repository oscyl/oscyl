//! `oscyl`: command-line tool for the synths Oscyl supports.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use oscyl_app::generator;
use oscyl_driver::{Driver, LibProgram, LibraryData, Macros, Recipe, SessionEvent, SessionOp};
use oscyl_midi::{DRIVERS, Device};
// User units, global and tuning data are minilogue xd matters the
// contract does not model; they run through `Device::run_units` on
// every logue synth (minilogue xd, prologue).
use oscyl_app::shelf::Shelf;
use oscyl_logue::osc;
use oscyl_logue::unit::{ParamKind, Unit, UnitInfo, palette_unit};
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
    /// Deal new sounds, each with a recipe code that makes it again.
    /// Needs no synth, except to --audition.
    Generate(Generate),
    /// Manage logue SDK user oscillators and effects (minilogue xd).
    Units {
        #[command(subcommand)]
        cmd: UnitsCmd,
    },
}

#[derive(clap::Args)]
struct Generate {
    /// List the kinds of sound the synth (--synth) can generate.
    #[arg(long)]
    list: bool,
    /// The kind of sound, by id (see --list).
    #[arg(long)]
    character: Option<String>,
    /// Filter openness, 0-100.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u8).range(0..=100))]
    bright: u8,
    /// Modulation, 0-100.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u8).range(0..=100))]
    movement: u8,
    /// Drive, noise and other dirt, 0-100.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u8).range(0..=100))]
    grit: u8,
    /// Delay, reverb and other ambience, 0-100.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u8).range(0..=100))]
    space: u8,
    /// The first sound's seed; the others follow from it. Random when
    /// left out.
    #[arg(long)]
    seed: Option<u64>,
    /// Deal the sound a recipe code names, first.
    #[arg(long, conflicts_with_all = ["character", "bright", "movement", "grit", "space", "seed"])]
    code: Option<String>,
    /// How many sounds [default: 6, or 1 with --code].
    #[arg(long)]
    count: Option<usize>,
    /// Write each sound into this folder as a program file.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Play the first sound on the synth, without storing it.
    #[arg(long)]
    audition: bool,
    /// Let the sounds use the user oscillators and effects the synth
    /// holds (about one sound in three): needs the synth, or --simulator.
    #[arg(long, conflicts_with_all = ["code", "vary", "breed"])]
    units: bool,
    /// Deal variations of the program in a file instead.
    #[arg(long, value_name = "FILE", conflicts_with_all = ["character", "code", "breed"])]
    vary: Option<PathBuf>,
    /// Deal children of the programs in two files instead.
    #[arg(long, num_args = 2, value_names = ["A", "B"], conflicts_with_all = ["character", "code"])]
    breed: Option<Vec<PathBuf>>,
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
    /// Show what a unit file says about itself: section, name and
    /// parameters. Needs no synth.
    Info { file: PathBuf },
    /// The shelf: unit files Oscyl keeps, ready to load.
    Shelf {
        #[command(subcommand)]
        cmd: ShelfCmd,
    },
    /// Make a new oscillator with Oscyl's oscillator unit and show it. Needs no
    /// synth, except to --load it.
    Make {
        /// What it is built around [default: any, by the seed].
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(
            ["wavetable", "fm", "fold", "formant", "noise", "hybrid"]))]
        family: Option<String>,
        /// The seed: the same seed and family make the same oscillator.
        /// Random when left out.
        #[arg(long)]
        seed: Option<u64>,
        /// Keep it: put it on the shelf and load it into the first free
        /// oscillator slot.
        #[arg(long)]
        load: bool,
        /// Write it as a unit file.
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum ShelfCmd {
    /// Copy unit files onto the shelf.
    Add {
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// List the shelf, each unit with its parameters.
    List,
    /// Take a unit off the shelf, by name.
    Remove { name: String },
    /// Load a unit of the shelf into the first free slot of its section.
    Load { name: String },
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
    /// Load slot data into the first free slot of its section.
    Load {
        module: Module,
        name: String,
        data: Vec<u8>,
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
        Cmd::Ports | Cmd::Generate(_) => unreachable!("handled before connecting"),
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
            UnitsCmd::Info { .. } => unreachable!("handled before connecting"),
            UnitsCmd::Make {
                family, seed, out, ..
            } => {
                let patch = make(family.as_deref(), seed, out.as_deref(), logue)?;
                let shelf = Shelf::at(&data_dir()?.join("units"));
                let unit = oscyl_app::oscillators::keep(&shelf, logue, &patch)
                    .map_err(|e| anyhow::anyhow!(e))?;
                (
                    Job::Load {
                        module: unit.module,
                        name: unit.name,
                        data: unit.data,
                    },
                    Some(logue),
                )
            }
            UnitsCmd::Shelf { cmd } => {
                let ShelfCmd::Load { name } = cmd else {
                    unreachable!("handled before connecting")
                };
                let shelf = Shelf::at(&data_dir()?.join("units"));
                let file = shelf_file(&shelf, logue, &name)?;
                let unit = shelf.unit(logue, &file).map_err(|e| anyhow::anyhow!(e))?;
                (
                    Job::Load {
                        module: unit.module,
                        name: unit.name,
                        data: unit.data,
                    },
                    Some(logue),
                )
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
    let cmd = match cli.cmd {
        Cmd::Generate(g) => return generate(g, chosen, cli.simulator, cli.port.as_deref()),
        Cmd::Units {
            cmd: UnitsCmd::Info { file },
        } => return unit_info(&file, chosen),
        Cmd::Units {
            cmd: UnitsCmd::Shelf { cmd },
        } if !matches!(cmd, ShelfCmd::Load { .. }) => return shelf(cmd, chosen),
        Cmd::Units {
            cmd:
                UnitsCmd::Make {
                    family,
                    seed,
                    load: false,
                    out,
                },
        } => {
            let d = chosen.unwrap_or_else(first_logue);
            return make(family.as_deref(), seed, out.as_deref(), d).map(|_| ());
        }
        cmd => cmd,
    };
    let (job, from_file) = prepare(cmd, chosen)?;
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

/// The synth a unit file is for: `chosen`, else the one whose unit files
/// have its extension.
fn unit_driver(path: &Path, chosen: Option<&'static dyn Driver>) -> Result<&'static dyn Driver> {
    let e = ext(path);
    chosen
        .or_else(|| {
            DRIVERS
                .iter()
                .copied()
                .find(|d| oscyl_midi::logue::unit_ext(*d) == Some(e.as_str()))
        })
        .with_context(|| format!("{}: no synth loads .{e} units", path.display()))
}

/// A unit's section, name and parameters, as lines.
fn print_unit(info: &UnitInfo) {
    println!("{:<6} {}", info.module.name(), info.name);
    for (i, p) in info.params.iter().flatten().enumerate() {
        // As the synth shows them: a choice reads one higher.
        let range = match p.kind {
            ParamKind::Percent => format!("{} % to {} %", p.min, p.max),
            ParamKind::Bipolar => format!("{:+} % to {:+} %", p.min, p.max),
            ParamKind::Select => format!("{} to {}", i16::from(p.min) + 1, i16::from(p.max) + 1),
        };
        println!("  {}  {:<12}  {range}", i + 1, p.name);
    }
}

/// `oscyl units info FILE`.
fn unit_info(path: &Path, chosen: Option<&'static dyn Driver>) -> Result<()> {
    let d = unit_driver(path, chosen)?;
    let p = platform(d).with_context(|| format!("the {} has no user units", d.info().model))?;
    let f = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let unit = Unit::read(&p, BufReader::new(f))
        .with_context(|| format!("cannot read {}", path.display()))?;
    print_unit(&unit.info());
    Ok(())
}

/// The first registered synth with user units.
fn first_logue() -> &'static dyn Driver {
    DRIVERS
        .iter()
        .copied()
        .find(|d| !d.capabilities().units.is_empty())
        .unwrap_or(DRIVERS[0])
}

/// `oscyl units make`: an oscillator of `family` (any, by the seed) for
/// `d`, shown, and written to `out` when given.
fn make(
    family: Option<&str>,
    seed: Option<u64>,
    out: Option<&Path>,
    d: &'static dyn Driver,
) -> Result<osc::Patch> {
    let p = platform(d)
        .filter(osc::supports)
        .with_context(|| format!("the {} cannot play Oscyl's oscillators", d.info().model))?;
    let seed = match seed {
        Some(s) => s,
        None => u64::from(SystemTime::now().duration_since(UNIX_EPOCH)?.subsec_nanos()),
    };
    let family = match family {
        Some(id) => osc::Family::from_id(id).context("no such family")?,
        None => osc::Family::ALL[(seed % osc::Family::ALL.len() as u64) as usize],
    };
    let patch = osc::generate(family, seed);
    let unit = osc::build(&p, &patch)?;
    print_unit(&unit.info());
    println!("  {} oscillator, seed {seed}", family.id());
    if let Some(out) = out {
        std::fs::write(out, osc::file(&p, &patch)?)
            .with_context(|| format!("cannot write {}", out.display()))?;
        eprintln!("wrote {}", out.display());
    }
    Ok(patch)
}

/// The file of the shelf unit called `name` (its name, or its file's).
fn shelf_file(shelf: &Shelf, d: &dyn Driver, name: &str) -> Result<String> {
    shelf
        .list(d)
        .into_iter()
        .find(|u| u.info.name == name || u.file == name)
        .map(|u| u.file)
        .with_context(|| {
            format!("no unit called {name:?} on the shelf; see `oscyl units shelf list`")
        })
}

/// `oscyl units shelf add|list|remove`: the shelf itself, no synth.
fn shelf(cmd: ShelfCmd, chosen: Option<&'static dyn Driver>) -> Result<()> {
    let shelf = Shelf::at(&data_dir()?.join("units"));
    let logue = || {
        chosen.unwrap_or_else(|| {
            DRIVERS
                .iter()
                .copied()
                .find(|d| !d.capabilities().units.is_empty())
                .unwrap_or(DRIVERS[0])
        })
    };
    match cmd {
        ShelfCmd::Add { files } => {
            for path in files {
                let d = unit_driver(&path, chosen)?;
                let bytes = std::fs::read(&path)
                    .with_context(|| format!("cannot open {}", path.display()))?;
                let added = shelf
                    .add(d, &bytes)
                    .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
                eprintln!(
                    "shelved \"{}\" ({})",
                    added.info.name,
                    added.info.module.name()
                );
            }
        }
        ShelfCmd::List => {
            for u in shelf.list(logue()) {
                print_unit(&u.info);
            }
        }
        ShelfCmd::Remove { name } => {
            let d = logue();
            let file = shelf_file(&shelf, d, &name)?;
            shelf.remove(d, &file).map_err(|e| anyhow::anyhow!(e))?;
            eprintln!("took \"{name}\" off the shelf");
        }
        ShelfCmd::Load { .. } => unreachable!("needs the synth"),
    }
    Ok(())
}

/// What the synth holds, as Generate's palette: every unit, and each
/// oscillator's parameters, read from its slot.
fn palette_of(device: &mut Device) -> Result<oscyl_driver::Palette> {
    let d = device.driver();
    let mut held = vec![];
    device.run_units(Op::ReadSlots, |e| {
        if let Event::Slot {
            module,
            slot,
            header: Some(h),
        } = e
        {
            held.push((*module, *slot, h.clone()));
        }
    })?;
    let mut units = vec![];
    for (module, slot, status) in held {
        let mut header = status;
        if module == Module::Osc {
            device.run_units(Op::ReadSlotData { module, slot }, |e| {
                if let Event::SlotData { data, .. } = e
                    && let Some(blob) = data.get(8..)
                {
                    header = blob.to_vec();
                }
            })?;
        }
        let kind = d
            .capabilities()
            .units
            .iter()
            .find(|k| k.name == module.name());
        if let (Some(kind), Ok(info)) = (kind, UnitInfo::from_header(&header)) {
            units.push(palette_unit(*kind, slot, &info));
        }
    }
    Ok(oscyl_driver::Palette { units })
}

/// The first program of a file, and the synth it is for (which must be
/// `chosen`, when one is).
fn first_program(
    path: &Path,
    chosen: Option<&'static dyn Driver>,
) -> Result<(&'static dyn Driver, Vec<u8>)> {
    let (d, progs) = programs(path)?;
    if let Some(c) = chosen.filter(|c| c.info().id != d.info().id) {
        bail!(
            "{} is for the {}, not the {}",
            path.display(),
            d.info().model,
            c.info().model
        );
    }
    let p = progs
        .into_iter()
        .next()
        .with_context(|| format!("{} has no programs", path.display()))?;
    Ok((d, p))
}

/// `base` and a number, within the synth's name length.
fn numbered(d: &dyn Driver, base: &str, k: usize) -> String {
    let tail = format!(" {k}");
    let room = d.name_rules().max_len.saturating_sub(tail.len());
    let head: String = base.trim_end().chars().take(room).collect();
    format!("{}{tail}", head.trim_end())
}

/// `oscyl generate`: deal sounds, print each one's name and recipe code,
/// and write or play them when asked.
fn generate(
    g: Generate,
    chosen: Option<&'static dyn Driver>,
    simulator: bool,
    port: Option<&str>,
) -> Result<()> {
    let fail = |e: oscyl_driver::DriverError| anyhow::anyhow!("{e}");
    // The clock when no seed is given; 32 bits, as a dealt seed is.
    let seed = match g.seed {
        Some(s) => s,
        None => u64::from(SystemTime::now().duration_since(UNIX_EPOCH)?.subsec_nanos()),
    };
    // Seeds for variations and children: SplitMix64's steps from `seed`.
    let step = |k: usize| seed.wrapping_add((k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let count = g.count.unwrap_or(if g.code.is_some() { 1 } else { 6 });
    let (d, sounds) = if let Some(file) = &g.vary {
        let (d, p) = first_program(file, chosen)?;
        let name = d.name(&p)?;
        let mut out = vec![];
        for k in 0..count {
            // The page's "Clear" amount.
            let mut v = d.vary(&p, generator::AMOUNTS[1], step(k)).map_err(fail)?;
            d.set_name(&mut v, &numbered(d, &name, k + 1))
                .map_err(fail)?;
            out.push((v, None, vec![]));
        }
        (d, out)
    } else if let Some(files) = &g.breed {
        let (d, a) = first_program(&files[0], chosen)?;
        let (_, b) = first_program(&files[1], Some(d))?;
        let name = d.name(&a)?;
        let mut out = vec![];
        for k in 0..count {
            // From mostly the first parent to mostly the second.
            let mix = 0.2 + 0.6 * (k as f32 + 0.5) / count as f32;
            let mut c = d.breed(&a, &b, mix, step(k)).map_err(fail)?;
            d.set_name(&mut c, &numbered(d, &name, k + 1))
                .map_err(fail)?;
            out.push((c, None, vec![]));
        }
        (d, out)
    } else {
        let d = chosen.unwrap_or(DRIVERS[0]);
        if g.list {
            for c in d.characters() {
                println!("{:<12} {:<7} {}", c.id, c.family.id(), c.hint);
            }
            return Ok(());
        }
        let recipe = match (&g.code, &g.character) {
            (Some(code), _) => generator::parse_code(d, code).map_err(|e| anyhow::anyhow!(e))?,
            (None, Some(id)) => {
                let id = id.to_lowercase();
                let character = d
                    .characters()
                    .iter()
                    .position(|c| c.id == id)
                    .with_context(|| {
                        format!(
                            "the {} has no {id:?} sounds; see `oscyl generate --list`",
                            d.info().model
                        )
                    })?;
                let dial = |v: u8| f32::from(v) / 100.0;
                Recipe {
                    character,
                    macros: Macros {
                        bright: dial(g.bright),
                        movement: dial(g.movement),
                        grit: dial(g.grit),
                        space: dial(g.space),
                    },
                    seed,
                    keep: vec![],
                    palette: Default::default(),
                }
            }
            (None, None) => bail!(
                "say what to generate: --character (see `oscyl generate --list`), --code, --vary or --breed"
            ),
        };
        // With --units the first sound may take one too: no seed is its own.
        let (mut recipe, mut first) = (recipe, (!g.units).then_some(seed));
        if g.code.is_some() {
            first = Some(recipe.seed);
        }
        if g.units {
            let mut device = if simulator {
                Device::demo(d)?
            } else {
                open(Some(d), port)?
            };
            recipe.palette = palette_of(&mut device)?;
            if recipe.palette.units.is_empty() {
                eprintln!("the synth holds no user units");
            }
        }
        let dealt = generator::deal(d, &recipe, None, recipe.seed, first, count).map_err(fail)?;
        let dealt = dealt.into_iter().map(|s| (s.program, s.code, s.units));
        (d, dealt.collect())
    };
    for (p, code, units) in &sounds {
        let with = match units.as_slice() {
            [] => String::new(),
            names => format!("  with {}", names.join(", ")),
        };
        let line = format!("{:<14}{}{with}", d.name(p)?, code.as_deref().unwrap_or(""));
        println!("{}", line.trim_end());
    }
    if let Some(dir) = &g.out {
        // The last format a driver lists is its smallest: a single
        // program, where it has one.
        let ext = d
            .file_formats()
            .last()
            .map_or(oscyl_core::native::EXT, |f| f.ext);
        let mut used: Vec<String> = vec![];
        for (p, ..) in &sounds {
            let name: String = d
                .name(p)?
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || " -_".contains(c) {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let mut file = name.clone();
            let mut k = 2;
            while used.contains(&file) {
                file = format!("{name}-{k}");
                k += 1;
            }
            let lib = LibraryData {
                programs: vec![LibProgram {
                    data: p.clone(),
                    ..LibProgram::default()
                }],
                ..LibraryData::default()
            };
            write_library(d, &lib, &dir.join(format!("{file}.{ext}")))?;
            used.push(file);
        }
        eprintln!("wrote {} programs to {}", sounds.len(), dir.display());
    }
    if g.audition
        && let Some((p, ..)) = sounds.first()
    {
        let mut device = if simulator {
            Device::demo(d)?
        } else {
            open(Some(d), port)?
        };
        run(Job::Audition { program: p.clone() }, &mut device)?;
    }
    Ok(())
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
        Job::Load { module, name, data } => {
            xd_only(device)?;
            let mut free = None;
            device.run_units(Op::ReadSlots, |e| {
                if let Event::Slot {
                    module: m,
                    slot,
                    header: None,
                } = e
                    && *m == module
                {
                    free = Some(free.map_or(*slot, |f: u8| f.min(*slot)));
                }
            })?;
            let Some(slot) = free else {
                bail!(
                    "every {} slot is full; clear one with `oscyl units clear`",
                    module.name()
                );
            };
            device.run_units(Op::UploadUnit { module, slot, data }, |_| {})?;
            eprintln!("loaded \"{name}\" into {} slot {}", module.name(), slot + 1);
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

/// GLib's user data directory, so the CLI and the desktop app share their
/// shelf and backups: `%LOCALAPPDATA%` on Windows (where `HOME` is usually
/// unset), `~/.local/share` elsewhere, `XDG_DATA_HOME` when set.
fn data_dir() -> Result<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = match var("XDG_DATA_HOME") {
        Some(d) => PathBuf::from(d),
        None if cfg!(windows) => {
            PathBuf::from(var("LOCALAPPDATA").context("LOCALAPPDATA is not set")?)
        }
        None => PathBuf::from(var("HOME").context("HOME is not set")?).join(".local/share"),
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
