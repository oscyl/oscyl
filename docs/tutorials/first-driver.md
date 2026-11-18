# Your first driver

> **Read this page in the built book.** Its code blocks come from the
> repository, and GitHub or an editor shows only the lines that include
> them. Read it on the website, at
> [oscyl.github.io/oscyl/docs/tutorials/first-driver.html](https://oscyl.github.io/oscyl/docs/tutorials/first-driver.html),
> or build the book yourself:
>
> 1. Install mdBook. On x86_64 Linux, `scripts/docs-tools.sh ~/.local/bin`
>    downloads the version that CI uses.
> 2. Run `mdbook build docs` in the repository.
> 3. Open `target/book/tutorials/first-driver.html`.

In this tutorial you write a driver for the Tutorial Mono, a small synth
that exists only in this tutorial. You build it one file at a time, test
each file, and finish when the driver passes Oscyl's conformance suite.

A driver teaches Oscyl one synth model. When a driver passes the suite,
every app can read the synth, edit its programs, play them and write them
back.

It takes about an hour.

## What you need

- A clone of the repository that builds:
  [Try Oscyl without a synth](try-without-a-synth.md) shows how.
- Some Rust. You copy every line of code from this page, but you read it
  too.

## The Tutorial Mono

A synth maker publishes what its synth sends and answers, in a MIDI
implementation chart. This is the chart of the Tutorial Mono.

- It has one bank of 16 program slots, numbered 0 to 15, and an edit
  buffer.
- A program is 14 bytes: an 8-character name in printable ASCII, padded
  with spaces, then one byte for each of six parameters.
- Every SysEx message starts with `F0 7D 54`. `7D` is the manufacturer ID
  that MIDI keeps for non-commercial use. `54` is the model.

The six parameters, in the order of their bytes after the name:

| Byte | Parameter | Group | Range | Control Change |
|---|---|---|---|---|
| 8 | Wave | Oscillator | 0 to 3: Saw, Square, Triangle, Noise | 70 |
| 9 | Glide | Oscillator | 0 (off) or 1 (on) | 65, sent as 0 or 127 |
| 10 | Cutoff | Filter | 0 to 127 | 74 |
| 11 | Resonance | Filter | 0 to 127 | 71 |
| 12 | Attack | Envelope | 0 to 127 | 73 |
| 13 | Release | Envelope | 0 to 127 | 72 |

The synth's messages:

| Message | Bytes | Answer |
|---|---|---|
| Program request | `F0 7D 54 01 slot F7` | A program dump of that slot |
| Program dump | `F0 7D 54 02 slot program F7` | The synth stores it, then sends an acknowledgement |
| Edit buffer request | `F0 7D 54 03 F7` | An edit buffer dump |
| Edit buffer dump | `F0 7D 54 04 program F7` | The synth plays it, then sends an acknowledgement |
| Acknowledgement | `F0 7D 54 7F slot F7` | Sent by the synth; `slot` is `7F` for the edit buffer |

To the MIDI device inquiry (`F0 7E 7F 06 01 F7`), the synth answers
`F0 7E channel 06 02 7D 54 4D 01 00 01 00 00 00 F7`:

- `06 02`: this is an identity reply;
- `7D`: the manufacturer;
- `54 4D` ("TM"): the family, the Tutorial Mono;
- `01 00`: the model in the family;
- `01 00 00 00`: the firmware version.

## 1. Start from an empty crate

The repository holds the finished driver in `crates/drivers/tutorial-synth`.
Delete it, so that you can rewrite it. Git keeps the finished files:
`git restore crates/drivers/tutorial-synth` brings them back at any time.

```sh
rm -r crates/drivers/tutorial-synth
mkdir -p crates/drivers/tutorial-synth/src crates/drivers/tutorial-synth/tests
```

The workspace includes every crate under `crates/drivers/`, so you do not
register the crate anywhere. The crate has three names, each for one use:

- the folder `tutorial-synth`;
- the package `oscyl-tutorial-synth`, which you give to Cargo with `-p`;
- the driver's id `oscyl-tutorial-mono`, which the apps and `--synth` use.

Create `crates/drivers/tutorial-synth/Cargo.toml`:

```toml
{{#include ../../crates/drivers/tutorial-synth/Cargo.toml}}
```

The driver depends only on `oscyl-driver`, the contract. The conformance
suite is a dev-dependency: only the tests use it.

Create `src/lib.rs`. It names the six files that you write in the next
steps:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/lib.rs}}
```

Create the six files, empty for now, and build:

```sh
cd crates/drivers/tutorial-synth
touch src/driver.rs src/files.rs src/msg.rs src/program.rs src/session.rs src/sim.rs
cd ../../..
cargo build -p oscyl-tutorial-synth
```

Cargo builds the crate and prints `Finished`. The crate is empty, but it
compiles: Rust accepts an empty module. Each file stays empty until its
step.

## 2. The program

Oscyl treats a program as bytes that only the driver understands. The apps
read and change a program only through the driver. Open `src/program.rs`.

First, the fields. A `FieldInfo` gives a parameter's name, the group that it
belongs to, and its smallest and largest raw value:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/program.rs:fields}}
```

Next, a check that refuses bytes that are not a whole program. Every other
function calls it first, so bad bytes never go further:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/program.rs:check}}
```

Then the functions that make a program and read or change one field:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/program.rs:values}}
```

Then the name:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/program.rs:name}}
```

Last, a test at the end of the file:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/program.rs:tests}}
```

Run it:

```sh
cargo test -p oscyl-tutorial-synth program
```

```text
test program::tests::a_field_and_the_name_read_back ... ok
```

## 3. The messages

Open `src/msg.rs`. One enum holds every message from the chart:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/msg.rs:messages}}
```

`encode` turns a message into bytes. `decode` does the opposite, and gives
`None` for any message that is not one of the Tutorial Mono's:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/msg.rs:encode}}
```

Oscyl knows a synth by its answer to the device inquiry. Add the answer and
a function that recognises it from any channel:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/msg.rs:identity}}
```

Add the test and run it:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/msg.rs:tests}}
```

```sh
cargo test -p oscyl-tutorial-synth msg
```

```text
test msg::tests::messages_round_trip ... ok
```

## 4. The simulator

A simulator is the synth in code. The conformance suite talks to it, and so
does the apps' demo mode. Open `src/sim.rs`. The synth's memory is 16
programs and an edit buffer:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/sim.rs:state}}
```

The synth's answer to each message follows the chart. A dump that holds a
bad program gets no answer, as on a real synth:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/sim.rs:answer}}
```

The `Simulator` trait is what Oscyl sees. The fault `lose_reply` makes the
synth silent. The suite switches it on to check that a lost reply never
counts as success:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/sim.rs:simulator}}
```

Add the test and run it:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/sim.rs:tests}}
```

```sh
cargo test -p oscyl-tutorial-synth sim
```

```text
test sim::tests::a_dump_is_stored_and_acknowledged ... ok
```

## 5. The session

The session is the protocol. It decides what to send for each operation and
which answer finishes it. It never touches a port or a clock: Oscyl gives it
the bytes and the time. This is *sans-IO*. The same session runs on every
platform and in tests.

The request engine in `oscyl_driver::engine` does the hard part. It keeps
one request in flight, times out a request whose reply does not come, and
reports progress. You describe the synth's protocol to it with the trait
`Protocol`:

| Item | What it is |
|---|---|
| `expand(op)` | Turns one operation, such as "read slots 3 and 4", into its requests, in order. |
| `on_message(bytes, expect, what, index)` | Says what one message from the synth means for the request in flight. |
| `Step` | One request: `msg`, the bytes to send; `expect`, what completes it; `timeout`; `what`, its name in error messages ("program write"); `index`, its slot, also for error messages. |
| `Outcome::Done` | The message completes the request, with an event for the apps or none. |
| `Outcome::Unrelated` | The message does not answer the request. The engine keeps waiting. |
| `Outcome::Failed` | The message fails the request, and the engine drops the rest of the operation. |
| `progress`, `done`, `failed` | Make the events that the engine reports. |

Open `src/session.rs`. First, write the requests that make up each
operation:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/session.rs:protocol}}
```

Notice that a write waits for the acknowledgement. The program dump alone
does not finish it.

Second, write what each message from the synth means for the request in
flight. `on_message` gets `what` and `index` so that an error can name
them. This driver's one error names the step only, so it ignores `index`.
Add the rest of the `impl` block, and a helper after it:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/session.rs:replies}}
```

A message that does not answer the request is `Unrelated`, and the engine
keeps waiting. If no answer comes, the request times out and fails.

The `DriverSession` trait is what Oscyl calls. The session checks every slot
and program before it queues anything, then passes each call to the engine:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/session.rs:session}}
```

Add the test and run it:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/session.rs:tests}}
```

```sh
cargo test -p oscyl-tutorial-synth session
```

```text
test session::tests::a_write_ends_when_the_synth_acknowledges_it ... ok
```

## 6. Files

A driver reads and writes the synth's own file formats, so that you can open
a file from the maker's software. The Tutorial Mono's `.syx` file is
program dumps, one after the other. Write `src/files.rs`:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/files.rs}}
```

```sh
cargo build -p oscyl-tutorial-synth
```

## 7. The driver

The `Driver` trait is all that the apps see of a synth. Open
`src/driver.rs`. First, the facts: the synth's identity, what it can do, and
its file format:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/driver.rs:statics}}
```

Start the `impl` block. These methods hand the program work to
`program.rs`:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/driver.rs:program}}
```

Add the file format:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/driver.rs:files}}
```

Add the editor. `curation` puts the fields on pages. Here
`curation_by_group` makes one page for each group: Oscillator, Filter and
Envelope. `control` and `format_raw` give Wave its four names:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/driver.rs:editor}}
```

Add live edits. With Live on, an app sends this Control Change for each
edit, so the change plays at once:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/driver.rs:live}}
```

Last, the link to the synth: the session, the simulator, the port name and
the identity. Close the `impl` block:

```rust
{{#include ../../crates/drivers/tutorial-synth/src/driver.rs:link}}
```

```sh
cargo build -p oscyl-tutorial-synth
```

## 8. Pass the conformance suite

Create `tests/conformance.rs`:

```rust
{{#include ../../crates/drivers/tutorial-synth/tests/conformance.rs}}
```

Run it:

```sh
cargo test -p oscyl-tutorial-synth --test conformance
```

```text
test tutorial_mono_conforms ... ok
```

The suite wrote programs to your simulator and read them back. It played a
program in the edit buffer, lost every reply on purpose, and checked that
nothing reported a false success. It also checked every field, every name,
the file format, the editor pages and the live messages.

When a check fails, its message names the driver, the check and the
problem, for example `oscyl-tutorial-mono: fields: Wave reads back wrong`.

## What you built

You wrote a complete driver:

- a program model;
- the synth's messages;
- a simulator;
- a sans-IO session on the request engine;
- a file format;
- the `Driver` that the apps see.

It passes the same suite as every driver of a real synth.

To put the repository's own copy back, run:

```sh
git restore crates/drivers/tutorial-synth
```

Next:

- [Add a driver for a new synth](../how-to/add-a-synth.md): from the first
  SysEx dump of a real synth to its hardware check.
- [The driver contract](../reference/driver-contract.md): every method, and
  the optional ones that this driver leaves out, such as graphs, help and
  Generate.
- [Architecture](../explanation/architecture.md): why drivers are sans-IO
  and compiled in.
