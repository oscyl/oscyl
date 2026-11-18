# The .oscyllib library format

Oscyl keeps each library in its own file, which no synth's software owns.
The code is
[`crates/oscyl-core/src/native.rs`](https://github.com/oscyl/oscyl/blob/main/crates/oscyl-core/src/native.rs).

A `.oscyllib` file is a ZIP archive with these members, in this order:

| Member | Content |
|---|---|
| `oscyl.json` | The manifest (below). |
| `programs/0000.bin`, `programs/0001.bin`, … | One program for each member, in library order, as raw bytes in the driver's own layout. The index has four digits and starts at 0. |
| `extras/<name>` | Members that the driver owns, kept byte for byte: vendor entries that Oscyl does not edit. For the minilogue xd: favourites, tuning tables, preset information, unknown members, and `FileInformation.xml`, which lists those entries. They let a later export to the vendor's format lose nothing. |

## Manifest

```json
{
  "format": 1,
  "driver": "korg-minilogue-xd",
  "programs": [
    { "author": "…", "comment": "…" }
  ]
}
```

- `format`: the version of the format.
- `driver`: the id of the driver that owns the program bytes and the extras
  (`Driver::info().id`).
- `programs`: one object for each program, in order. `programs[i]`
  describes `programs/<i>.bin`. Every program in the list has its member.

## Versioning

- A reader refuses a file whose `format` is higher than the newest that it
  knows. It says that a newer version of Oscyl made the library.
- A new version can add optional fields without a new `format`. Readers
  ignore fields that they do not know, and a missing optional field takes
  its default.
- A change that an older reader would misread raises `format`.

## Migration from `.mnlgxdlib`

Before this format, the Linux app kept its libraries as minilogue xd
`.mnlgxdlib` files, listed in `index.json`. The index is now
`libraries.json`. When it does not exist yet, Oscyl starts it from
`index.json`, and leaves `index.json` in place for an older build.

When the library folder opens, Oscyl takes each old file through these
steps:

1. It converts the file to `.oscyllib`.
2. It reads the new file back and compares it with the conversion.
3. It moves the original into `before-oscyllib/`, with a copy of the
   original `index.json`.

If a step fails:

- **Oscyl cannot read the original:** the original moves into the backup
  folder as it is.
- **Oscyl cannot write the new file, or it does not read back equal:** the
  original stays, with its entry in the index. The next open tries again.
- **A backup with the same name exists:** Oscyl keeps the original as
  `<id>.<n>.mnlgxdlib` instead.

Oscyl never deletes or overwrites an original.
