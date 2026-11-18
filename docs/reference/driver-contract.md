# The driver contract

The traits that a driver implements, in
[`crates/oscyl-driver/src/contract.rs`](https://github.com/oscyl/oscyl/blob/main/crates/oscyl-driver/src/contract.rs).
Each method links to its documentation in the
[API reference](https://oscyl.github.io/oscyl/api/oscyl_driver/index.html).
[ADR 0002](../adr/0002-driver-contract.md) records why the contract is
made this way.

A driver is a crate under `crates/drivers/` that exposes one
`pub static DRIVER`. The apps use it only as `&dyn Driver`. A required
method has no default. An optional method has a default that suits a
driver which does not use it.

## [`Driver`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html)

One synth model: everything that the apps know about it.

### Identity

| Method | Required | What it gives |
|---|---|---|
| [`info`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.info) | Yes | The driver's id, the maker and the model. |
| [`recognize`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.recognize) | Yes | Whether an answer to the MIDI device inquiry comes from this synth. |
| [`port_hint`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.port_hint) | Yes | Substrings of the names of the MIDI ports that the synth shows. |
| [`firmware`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.firmware) | No | The firmware version that an identity reply names. Default: none. |
| [`setup_hint`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.setup_hint) | No | What to check on the synth when it does not answer the inquiry. Default: empty. |
| [`tier`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.tier) | No | The driver's support tier. Default: 3. |
| [`voices`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.voices) | No | The polyphony of the models that it covers. Default: unknown. |

### What the synth has

| Method | Required | What it gives |
|---|---|---|
| [`capabilities`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.capabilities) | Yes | The banks and their slots, which banks are writable, the edit buffer, live changes and unit slots. |

### The program

| Method | Required | What it gives |
|---|---|---|
| [`fields`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.fields) | Yes | The parameters: name, group and range. |
| [`init_program`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.init_program) | Yes | A new, valid program. |
| [`check_program`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.check_program) | Yes | Refuses bytes that are not a whole program. |
| [`get`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.get) | Yes | A field's raw value. |
| [`set`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.set) | Yes | Sets one field, and no other. |
| [`name`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.name) | Yes | The program's name. |
| [`set_name`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.set_name) | Yes | Sets the name. |
| [`name_rules`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.name_rules) | Yes | The longest name and the characters that a name can hold. |

### Files

| Method | Required | What it gives |
|---|---|---|
| [`file_formats`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.file_formats) | Yes | The synth's own file formats. Can be empty. |
| [`import`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.import) | Yes | Reads a file of one of those formats. |
| [`export`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.export) | Yes | Writes a file. Importing it gives back the same library. |

### The editor

| Method | Required | What it gives |
|---|---|---|
| [`curation`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.curation) | Yes | The editor's pages and sections. A simple driver can return `curation_by_group(self.fields())`. |
| [`control`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.control) | No | The control for a field: a dial, a switch, named choices or named bands. Default: a switch for a two-value field, else a dial. |
| [`short_label`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.short_label) | No | A short label inside a titled section. Default: the field's name. |
| [`label_in`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.label_in) | No | The label when a field's job depends on another field. Default: the short label. |
| [`emphasized`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.emphasized) | No | Whether a field is a main control, which apps draw larger. Default: no. |
| [`format_value`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.format_value) | No | A field's value as text, in the context of the program. Default: `format_raw` of the value. |
| [`format_raw`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.format_raw) | No | A raw value as text, with no program. Default: the number, or Off and On for a two-value field. |
| [`page_summary`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.page_summary) | No | One line on a page's state. Default: empty. |
| [`page_summary_in`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.page_summary_in) | No | The page summary, short or translated, for an app. Default: `page_summary`. |
| [`graphs`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.graphs) | No | Pictures of a page. Default: none. |
| [`handles`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.handles) | No | Draggable points on a page's first graph. Default: none. |
| [`drag`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.drag) | No | The field values for a handle at a point. Default: none. |
| [`relevant`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.relevant) | No | Whether a field does anything with the other settings. Default: yes. |
| [`help`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.help) | No | What a field does. Default: empty. |
| [`category`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.category) | No | What kind of sound a program is. Default: unknown. |
| [`categories`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.categories) | No | Every category, for filters. Default: none. |
| [`list_facet`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.list_facet) | No | A choice field that program lists can filter by. Default: none. |

### Generate

| Method | Required | What it gives |
|---|---|---|
| [`characters`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.characters) | No | The kinds of sound that Generate makes. Default: none, so the driver does not generate. |
| [`macro_info`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.macro_info) | No | The titles and help of the four dials. Default: Bright, Movement, Grit and Ambience. |
| [`mutability`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.mutability) | No | What generation may do to a field. Default: keep it. |
| [`generate`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.generate) | No | A new program from a recipe. |
| [`vary`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.vary) | No | A variation of a program. |
| [`breed`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.breed) | No | A child of two programs. |
| [`morph`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.morph) | No | A step between two programs. Default: `morph_fields`. |
| [`generated_name`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.generated_name) | No | A name for a generated program. Default: the character's title and a number. |
| [`needs`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.needs) | No | The user unit slots that a program relies on. Default: none. |
| [`unit_param`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.unit_param) | No | Which user unit parameter a field is. Default: none. |

### Live and the panel

| Method | Required | What it gives |
|---|---|---|
| [`live_messages`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.live_messages) | No | The MIDI messages that set one field on the sounding voices. Default: none, so apps send the whole program. |
| [`program_slot`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.program_slot) | No | The slot that the synth's panel picked, from Bank Select and Program Change. Default: unknown. |
| [`panel_listener`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.panel_listener) | No | A decoder for the Control Changes that the panel sends. Default: none. |

### Sessions and demo

| Method | Required | What it gives |
|---|---|---|
| [`session`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.session) | Yes | A new session for one synth. |
| [`simulator`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#tymethod.simulator) | Yes | A new simulated synth. |
| [`demo_library`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.demo_library) | No | A library to explore without a synth. Default: one generated program for each character. |
| [`demo_simulator`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Driver.html#method.demo_simulator) | No | The simulated synth of demo mode. Default: `simulator`. |

## [`DriverSession`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.DriverSession.html)

The synth's protocol for one connected synth, sans IO: it never touches a
port or a clock. Oscyl gives it the bytes and the time. Most sessions keep
an [`Engine`](https://oscyl.github.io/oscyl/api/oscyl_driver/engine/struct.Engine.html), the request engine,
which keeps one request in flight. Every method is required.

| Method | What it does |
|---|---|
| [`submit`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.DriverSession.html#tymethod.submit) | Plans an operation. A bad slot, a write to a factory slot or a bad program fails here, and nothing is sent. |
| [`is_idle`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.DriverSession.html#tymethod.is_idle) | True when nothing is queued or in flight. |
| [`poll_transmit`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.DriverSession.html#tymethod.poll_transmit) | The next bytes to send, if the session sends now. |
| [`next_deadline`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.DriverSession.html#tymethod.next_deadline) | When a request in flight times out. |
| [`handle_timeout`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.DriverSession.html#tymethod.handle_timeout) | Time passed: a request whose reply did not come fails. |
| [`handle_input`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.DriverSession.html#tymethod.handle_input) | Bytes from the synth, in fragments of any size. |
| [`poll_event`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.DriverSession.html#tymethod.poll_event) | The next result, in order. |

An operation is a [`SessionOp`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/enum.SessionOp.html):
`ReadCurrent`, `ReadPrograms`, `WritePrograms` or `Audition`. Its results
are [`SessionEvent`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/enum.SessionEvent.html)s: `Current`,
`Program`, `Progress`, then `Done` or `Failed`.

## [`Simulator`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Simulator.html)

The synth itself, in code, sans IO. The conformance suite and demo mode
talk to it. Every method is required.

| Method | What it does |
|---|---|
| [`handle`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Simulator.html#tymethod.handle) | One message in, the synth's replies out. |
| [`idle_tick`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Simulator.html#tymethod.idle_tick) | One pass of the firmware's main loop, with no MIDI traffic. |
| [`program`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Simulator.html#tymethod.program) | The program in a slot. |
| [`edit_buffer`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Simulator.html#tymethod.edit_buffer) | The edit buffer's program. |
| [`is_settled`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Simulator.html#tymethod.is_settled) | No save is pending and no reply state is corrupt. |
| [`faults`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Simulator.html#tymethod.faults) | The faults seen on the hardware that `inject` can switch on. It must include `lose_reply`. |
| [`inject`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.Simulator.html#tymethod.inject) | Switches a fault on. |

## [`PanelListener`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.PanelListener.html)

An optional decoder of the Control Changes that a synth's panel sends,
from `Driver::panel_listener`. Its one method,
[`control`](https://oscyl.github.io/oscyl/api/oscyl_driver/contract/trait.PanelListener.html#tymethod.control), gives the field and the value that a
Control Change sets, once the value is complete.

## What the conformance suite checks

`oscyl_conformance::run(&DRIVER)` runs these checks in order, against the
driver's simulator. A failure panics with `<driver id>: <check>: <what>`.

| Check | What it proves |
|---|---|
| `init_program` | The init program is valid, and its name is not empty and uses allowed characters. |
| `fields` | Every field takes its minimum and its maximum, reads them back, and changes no other field. One step outside the range is refused. |
| `names` | A name of the longest length reads back. One character more is refused. |
| `wrong_length_programs` | A program one byte short or one byte long is refused by every method that takes a program. |
| `rejects_bad_targets` | A write to a factory slot or past the last slot, and a read past the last slot, fail at `submit` and send nothing. |
| `file_formats_round_trip` | Every file format gives back the programs, and the author and comment when it keeps them. A second round trip changes nothing. An unknown format is refused. |
| `write_then_read_back` | A write to the first, second and last slot of each writable bank ends `Done`. The simulator then holds the programs and is settled, and a read gives them back. |
| `audition` | With an edit buffer: an audition ends `Done`, the edit buffer holds the program, and `ReadCurrent` gives it back. |
| `progress` | Reads and writes report progress that rises to the total, and `Done` comes last. |
| `faults_never_report_false_success` | With each fault: a write that ends `Done` stored every program, and a read returns only what the synth holds. |
| `lost_replies_fail` | The simulator has `lose_reply`. With it, every operation ends `Failed`. |
| `curation_covers_every_field` | Every field is in exactly one section or excluded, and at least one is shown. Every shown field has a label and a control that fits it. |
| `formats_every_value` | Every field's minimum and maximum format as text that is not empty. |
| `generation_is_deterministic_and_valid` | For each character, the same recipe gives the same bytes, and every value is in range. Kept fields and pages stay, and vary, breed and morph behave. |
| `live_messages_are_well_formed` | Live messages exist only with `live_changes`, and each one is one complete channel message on the channel asked for. |
| `presentation_is_consistent` | Emphasized fields are shown, labels exist, categories are listed, and graphs, handles and drags stay within the pages and fields. |
| `recognizes_its_simulator` | Port hints exist. The driver recognises its simulator's identity reply from any channel, but not the inquiry itself and not another maker's reply. |
| `app_hooks` | The demo library is valid and fits the writable slots, the demo simulator is recognised, and the list facet is an in-range choice field. |
