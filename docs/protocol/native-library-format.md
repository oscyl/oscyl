# Native library format (`.oscyllib`)

Oscyl stores each library in its own file, independent of any synth. The
code is `crates/oscyl-core/src/native.rs`.

A `.oscyllib` file is a ZIP archive with these members, in this order:

| Member | Content |
|---|---|
| `oscyl.json` | Manifest (below) |
| `programs/0000.bin`, `programs/0001.bin`, … | One program per member, in library order, as raw bytes in the driver's native layout. Four-digit, zero-based index. |
| `extras/<name>` | Driver-owned members, verbatim: vendor entries Oscyl does not edit (for the minilogue xd: favourites, tuning tables, preset information, unknown members, and `FileInformation.xml` listing those entries). They make a later export to the vendor format lossless. |

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

- `format`: the format version.
- `driver`: the id of the driver that owns the program bytes and extras
  (`Driver::info().id`).
- `programs`: one object per program, in order. `programs[i]` describes
  `programs/<i>.bin`. Every listed program must have its member.

## Versioning

- A reader refuses a file whose `format` is higher than the newest it knows,
  with "made by a newer version of Oscyl".
- New optional fields may be added without a version change. Readers ignore
  fields they do not know, and missing optional fields take defaults.
- Any change an older reader would misread raises `format`.

## Migration from `.mnlgxdlib`

Before this format, the Linux app stored libraries as minilogue xd
`.mnlgxdlib` files, listed in `index.json`. Now the index is
`libraries.json`. On first open it starts from `index.json`, which is left
in place so an older build still finds its own index.

When the library folder opens, each legacy file is handled like this:

1. It is converted to `.oscyllib`.
2. The new file is read back and compared with the conversion.
3. The original moves into `before-oscyllib/`, along with a copy of the
   original `index.json`.

If a step fails:

- **The original cannot be read:** it moves into the backup unchanged.
- **The new file cannot be written or does not read back equal:** the
  original stays where it is, and the next open tries again. Its index
  entry is kept.
- **A backup with the same name already exists:** the original is kept as
  `<id>.<n>.mnlgxdlib` instead.

Originals are never deleted or overwritten.
