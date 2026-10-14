# Export tag and file information

Choose **File → Export…**, **Ctrl E**, or **Cmd E** on macOS. Configuration management works without a file selection. Select files and save pending tag edits before exporting.

Use **New**, **Duplicate**, **Remove**, or **Import .mte…** to manage configurations. Edit the name and template in the dialog. F2 focuses the name; F4 focuses the template. Import accepts UTF-8 and BOM-marked UTF-16. Supply a legacy encoding when needed. **Save configurations** remembers the library and options; **Cancel** discards unsaved dialog changes. **Export** saves the current configuration and options before running it.

Enter an output filename or choose **Browse…**. Relative names use the first selected file's folder. Leave it blank to use the configuration's `$filename()` statement or `export.txt`. Filename expressions can include metadata and scripting functions.

Enable **Create one output file per** and enter a partition expression, such as `%album%`. Files with the same result share an output. Use a filename expression that differs between partitions, such as `reports/%album%.txt`.

**Append data** adds to existing output. **Write BOM** applies to UTF-8 and UTF-16; appending to a nonempty file does not add another BOM. Disable **Create output directories** to require existing parent folders. BOM and directory choices also apply to Export actions; actions retain their own filenames, append and partition settings.

Templates are UTF-8 `.mte` files under the application's configuration folder at `export/<configuration-id>.mte`. Names, identities and options are in `exports.json`. External template edits load when the dialog reopens. Syntax is checked before saving; unfinished external edits can be opened for repair. Removing a saved configuration moves its MTE to `export/removed/` when saved. Export actions can reference configurations by displayed name or MTE path.

Export leaves input audio unchanged. Output files are committed individually; an I/O error can leave earlier outputs written. See the [verification record](verification/export-manager.md) and [Mp3tag export reference](https://docs.mp3tag.de/export/) for evidence and remaining differences.
