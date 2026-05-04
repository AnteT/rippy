# Rippy CLI Tool

`rippy` is an OS-independent bastard love child of `tree` and `grep` with subdirectory pruning, `.gitignore` support, advanced filtering, and multithreaded file pattern search.

## Features

- **Directory Tree Display**: Visualizes directory contents in a hierarchical tree format.
- **Multithreaded Search**: Searches file contents in parallel using `rayon`.
- **Regex Search**: Search file contents with regular expressions.
- **Logical Search Operators**: Chain search terms with `--and` and `--or`.
- **Search Windowing**: Shows snippets of file content around matching patterns by default.
- **Line Numbers**: Displays matching line numbers as `path:line`.
- **Multiple Matches Per File**: Display every match in a file with `--all-matches`, or cap results with `--max-display-matches`.
- **Advanced Filtering**: Use `.gitignore` files, ignore patterns, include patterns, hidden file filtering, and directories-only output.
- **Sorting Results**: Sort by name, date, size, or type, including reverse order.
- **Customizable Output**: Display flat lists, full paths, relative paths, sizes, dates, enumerated entries, quoted paths, and more.
- **Output to JSON**: Export tree and search results to a JSON file.
- **Pretty Errors**: Invalid regexes and bad inputs now return readable error messages instead of panicking.

---

## Installation

To install `rippy`, clone the repository and build it with Cargo:

```sh
$ git clone https://github.com/AnteT/rippy.git
$ cd rippy && cargo build --release
````

### Linux and macOS

After building, the executable will be located at `./target/release/rippy`. To make it globally accessible:

```sh
$ sudo cp target/release/rippy /usr/local/bin
```

Confirm the installation:

```sh
$ rippy --version
```

### Windows

After building, the executable will be located at `.\target\release\rippy.exe`.

To make it globally accessible, add the release directory to your Windows `PATH`:

```cmd
> cd .\target\release\
> echo %cd%
```

Then add the printed directory to your system `PATH` using **Environment Variables**.

Verify installation in a new terminal:

```cmd
> rippy --version
```

---

## Usage

```sh
$ rippy [DIRECTORY] [PATTERN] [OPTIONS]
```

### Arguments

* `[DIRECTORY]`: Root directory to search. Defaults to `.`.
* `[PATTERN]`: Optional regex pattern to search file contents for.

You can also provide the search pattern with:

```sh
$ rippy . --search "TODO"
$ rippy . --find "TODO"
$ rippy . -f "TODO"
```

---

## Optional Flags

### Search

* `-f, --search, --find <PATTERN>`: Search file contents for a regex pattern
* `--and <PATTERN>`: Require an additional pattern to match in the same file
* `--or <PATTERN>`: Add an alternative pattern group
* `-C, --case-insensitive`: Make pattern matching case-insensitive
* `-W, --windowless`: Display search results without snippet windows
* `-R, --window-radius <RADIUS>`: Maximum character radius for result snippets
* `--all-matches`: Display every matching window for each file
* `-m, --max-display-matches <COUNT>`: Maximum number of match windows to display per file
* `--no-line-numbers`: Hide line numbers from search results

### Traversal and Filtering

* `-A, --all`: Include hidden files and directories
* `-g, --no-gitignore`: Do not use `.gitignore` files when found
* `-I, --ignore <PAT1, ..., PATN>`: Ignore specific file or directory patterns
* `-X, --include <PAT1, ..., PATN>`: Restrict files to matching filename patterns; directories may also match for tree output
* `-L, --max-depth <DEPTH>`: Maximum directory depth to search
* `-l, --follow-links`: Follow symbolic links
* `--dirs, --dirs-only, --directories-only`: Display only directories

### Display

* `-B, --sort-by <KEY>`: Sort by `date`, `name`, `size`, or `type`
* `-Z, --reverse`: Reverse sort order
* `-M, --max-files <FILES>`: Maximum number of files to display per directory
* `-N, --indent <WIDTH>`: Character width for tree indentation
* `-P, --relative-path`: Display relative paths from root
* `-K, --full-path`: Display full canonical paths
* `-S, --size`: Display file sizes
* `-D, --date`: Display last modified timestamps
* `-Y, --date-format <FORMAT>`: Format dates using a chrono/strftime format
* `-U, --dir-detail`: Display size and date details for directories
* `-E, --enumerate`: Enumerate entries within each parent
* `-Q, --quote`: Wrap displayed paths in double quotes
* `-F, --flat`: Display results as a flat list
* `-G, --gray`: Display without color styling
* `-T, --time`: Display elapsed runtime
* `-J, --just-counts`: Display only summary counts

### Output

* `-O, --output <FILENAME>`: Export results as JSON

### Help and Version

* `-h, --help`: Display help and usage information
* `-v, --version`: Display the version of `rippy`

---

## Examples

### Display the current directory
If no directory is provided, `rippy` will display the current directory's contents minus any ignored patterns found in files like `.gitignore`, for example in this project's root directory running:

```sh
$ rippy
```

Results in the below output:

```text
 .
 ├── Cargo.lock
 ├── Cargo.toml
 ├── LICENSE
 ├── README.MD
 ├── build.rs
 ╰── src
     ├── args.rs
     ├── bin
     │   ╰── rippy.rs
     ├── crawl.rs
     ├── error.rs
     ├── lib.rs
     ├── tcolor.rs
     ╰── tree.rs

2 directories, 12 files
```

### Limit traversal depth

```sh
$ rippy . -L 1
```

Shows only immediate children of the root directory.

```text
 .
 ├── Cargo.lock
 ├── Cargo.toml
 ├── LICENSE
 ├── README.MD
 ├── build.rs
 ╰── src

1 directory, 5 files
```

### Search for a pattern

```sh
$ rippy . "usize"
```

Search results are displayed as `path:line` followed by a snippet.

```text
 .
 ├── README.md:195 ┆ $ rippy . "usize"
 ├── PROMPT.md:138 ┆ ...pub groups: Vec<Vec<usize>>,
 ╰── src
     ├── args.rs:59  ┆ ...pub groups: Vec<Vec<usize>>,
     ├── crawl.rs:44 ┆ ...line_number: Option<usize>,
     ╰── tree.rs:89  ┆ ...b fmt_width: Option<usize>,

5 matches, 14 searched
```

### Search with explicit `--search`

```sh
$ rippy . --search "pub struct"
```

Equivalent to:

```sh
$ rippy . "pub struct"
```

### Search with logical operators

```sh
$ rippy . "needle0" --and "needle1" --or "needle2"
```

This behaves like:

```text
(needle0 AND needle1) OR needle2
```

### Show all matches in each file

```sh
$ rippy . "pub struct" --all-matches
```

Example output:

```text
 .
 ╰── src
     ├── args.rs:51   ┆ pub struct SearchTerm {
     │   args.rs:57   ┆ pub struct SearchQuery {
     │   args.rs:95   ┆ pub struct RippyArgs {
     ├── crawl.rs:14  ┆ pub struct Ignorer {
     │   crawl.rs:43  ┆ pub struct MatchWindow {
     │   crawl.rs:58  ┆ pub struct TreeLeaf {
     │   crawl.rs:105 ┆ pub struct CrawlResults {
     ├── tcolor.rs:71 ┆ pub struct RippySchema {
     ╰── tree.rs:61   ┆ pub struct TreeIter<'a> {
         tree.rs:81   ┆ pub struct Tree {
         tree.rs:321  ┆ pub struct TreeCounts {

4 matches, 12 searched
```

### Limit displayed matches per file

```sh
$ rippy . "pub" --max-display-matches 3
```

### Search only Rust files

```sh
$ rippy . "JSON" -X "*.rs"
```

### Search with size and date details

```sh
$ rippy . "JSON" -S -D -X "*.rs"
```

```text
 .
 ╰── src
     ├── (2026-04-27 22:41:13,  31 K) args.rs:433  ┆ ...port the results as JSON to specified file")...
     ╰── (2026-04-27 17:58:42, 6.3 K) error.rs:128 ┆ ...r!("Failed to write JSON output [", &path_fm...

2 matches, 8 searched
```

### Use a custom snippet radius

```sh
$ rippy . "registry" -R 40
```

### Hide snippets but keep line numbers

```sh
$ rippy . "pub struct" --windowless
```

```text
 .
 ╰── src
     ├── args.rs:51
     ├── crawl.rs:39
     ╰── tree.rs:62

3 matches, 13 searched
```

### Hide line numbers

```sh
$ rippy . "pub struct" --no-line-numbers
```

### Display only directories

```sh
$ rippy . --dirs
```

or:

```sh
$ rippy . --directories-only
```

### Search and return only directories containing matches

```sh
$ rippy . "TODO" --dirs
```

This searches files, but displays only directories containing matching files.

### Include only matching names

```sh
$ rippy . -x "src"
```

This can match directories such as `src`.

### Ignore directories or files

```sh
$ rippy . -I ".git,target"
```

### Include hidden files but ignore `.git`

```sh
$ rippy . --all -I ".git"
```

### Sort results

```sh
$ rippy . --sort size
$ rippy . --sort date --reverse
$ rippy . --sort type
```

Sort keys:

* `date`: Sort by last modified date
* `name`: Sort alphabetically by entry name
* `size`: Sort by size in bytes
* `type`: Sort by entry type

### Export to JSON

```sh
$ rippy . --find "copyright" --all-matches --case-insensitive -O results.json
```

Example JSON shape:

```json
{
  "name": ".",
  "entry_type": "Directory",
  "last_modified": null,
  "size": null,
  "window": null,
  "windows": [],
  "children": [
    {
      "name": "LICENSE",
      "entry_type": "File",
      "last_modified": "2024-11-11 21:51:35",
      "size": 1095,
      "window": "3 Copyright (c) 2024 Ante Tonkovic-Capin...",
      "windows": [
        "3 Copyright (c) 2024 Ante Tonkovic-Capin...",
        "12 The above copyright notice and this per...",
        "18 AUTHORS OR COPYRIGHT HOLDERS BE LIABLE F..."
      ],
      "children": []
    }
  ]
}
```

---

## Search Behavior Notes

* Search patterns are regular expressions.
* Invalid regular expressions return pretty error messages instead of panicking.
* File content search runs in parallel.
* By default, one match window is displayed per file.
* Use `--all-matches` to show every match in each file.
* Use `--max-display-matches <COUNT>` to cap match windows per file.
* Search output uses `path:line` formatting when line numbers are enabled.
* The snippet separator can be adjusted in `tree.rs` with `SEARCH_SNIPPET_SEPARATOR`.

---

## Filtering Notes

* Hidden files and directories are excluded by default.
* Use `--all` to include hidden entries.
* `.gitignore` files are respected by default.
* Use `--no-gitignore` to disable `.gitignore` filtering.
* `--ignore` and `--include` accept comma-delimited patterns.
* Patterns containing `*` are treated as wildcard-style patterns and converted to regex sets.
* Directories are preserved during traversal so `--max-depth` and include filters behave correctly.

---

## License

This project is [MIT licensed](https://github.com/AnteT/rippy/blob/master/LICENSE), with the source code for `rippy` openly available at [github.com/AnteT/rippy](https://github.com/AnteT/rippy).

Thank you!

*- Ante Tonkovic-Capin*
