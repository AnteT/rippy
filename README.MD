# Rippy CLI Tool
`rippy` is an OS independent bastard love child of `tree` and `grep` with subdirectory pruning and multithreaded file pattern search.

## Features
- **Directory Tree Display**: Visualizes directory contents in a hierarchical tree format.
- **Multithreaded Search**: Perform fast searches for patterns within files using `rayon` to parallelize and leverage multiple threads when possible.
- **Advanced Filtering**: Filter entries with options to use any `.gitignore` files found for matching, or provide your own ignore or include patterns to use.
- **Sorting Results**: Sort the tree output by name, date, size or type including in reverse order.
- **Search Windowing**: Optionally shows snippets of file content around matching patterns.
- **Customizable Output**: Display results as flat lists, include file size, dates, and more.
- **Output to JSON**: Export the tree and search results to a file.

---

## Installation
To install `rippy`, clone the repository and build it with `Cargo`:

```sh
$ git clone https://github.com/AnteT/rippy.git
$ cd rippy && cargo build --release
```

### Linux and macOS
1. After building `rippy`, the executable will be located in `./target/release/rippy`, to save it to your path to make it globally accessible run:

    ```sh
    $ sudo cp target/release/rippy /usr/local/bin
    ```

2. Confirm the installation process was succesful by running:

    ```sh
    $ rippy --version
    ```

### Windows
After building `rippy`, the executable will be located in `.\target\release\rippy.exe`, to make it globally accessible you can add it to your Windows `PATH` using these steps:

1.  Navigate to the location containing `rippy` using the same PowerShell or Command Prompt session that was used to build it:

    ```cmd
    > cd .\target\release\
    ```

2. Copy the full path of the installation by grabbing it with:

    ```cmd
    > echo %cd%
    ```

3. Add the directory containing `rippy.exe` to your system `PATH`:
   - Open the **Start Menu**, search for **Environment Variables**, and select **Edit the system environment variables**.
   - In the **System Properties** window, click **Environment Variables**.
   - Under **System variables**, select **Path** and click **Edit**.
   - In the **Edit Environment Variables** window, click **New** and paste the path you copied (e.g., `C:\path\to\rippy\target\release`).
   - Click **OK** to save the changes.

4. To verify that `rippy` is accessible globally, open a new PowerShell or Command Prompt and run:

   ```cmd
   > rippy --version
   ```

## Usage
To use `rippy`, simply run it from the command line:

```sh
$ rippy <DIRECTORY> [PATTERN] [OPTIONS]
```

### Arguments
- `<DIRECTORY>`: The root directory to search, the sole required argument.
- `[PATTERN]`: Optional pattern to search file contents for (supports regular expressions).

### Optional Flags
- `-A, --all`: Include hidden files and directories
- `-B, --sort-by <KEY>`: Sorting options: 'date', 'name' [d], 'size' or 'type'
- `-L, --max-depth <DEPTH>`: Maximum directory depth to search
- `-Y, --date-format <FORMAT>`: Display date using the specified format (e.g., '%Y-%m-%d'), see [chrono docs](https://docs.rs/chrono/latest/chrono/format/strftime/index.html) for more details
- `-I, --ignore <PAT1, ..., PATN>`: Ignore specific file extensions or directories
- `-X, --include <PAT1, ..., PATN>`: Restrict search to specific filename patterns
- `-R, --window-radius <RADIUS>`: Maximum character radius for result snippet window
- `-M, --max-files <FILES>`: Maximum number of files to display for each directory
- `-O, --output <FILENAME>`: Export the results as JSON to specified file
- `-N, --indent <WIDTH>`: Character width to use for tree depth indentation
- `-C, --case-insensitive`: Make pattern matching case insensitive
- `-l, --follow-links`: Follow targets of symbolic links when found
- `-P, --relative-path`: Display the relative paths from root with results
- `-Z, --reverse`: Reverses sort order from ascending to descending
- `-K, --full-path`: Display the full canonical paths with results
- `-S, --size`: Display the size of files and directories with results
- `-D, --date`: Display the system last modified datetime with results
- `-E, --enumerate`: Display results enumerated by index within parent
- `-T, --time`: Display the search duration time with results
- `-g, --no-gitignore`: Do not use .gitignore files when found for filtering
- `-G, --gray`: Display the results in grayscale without styling
- `-Q, --quote`: Display the path results wrapped in double-quotes
- `-F, --flat`: Display the results as flat list without indentation
- `-U, --dir-detail`: Display size and date time details for directories
- `-W, --windowless`: Display search results without context snippet window
- `-J, --just-counts`: Display just entry counts without rendering a tree
- `-h, --help`: Display help and usage information for rippy
- `-v, --version`: Display the version of rippy

## Examples
### Quick Overview
Some example use-cases for `rippy` are provided below:

1. **Display a tree of the current directory**:
   ```sh
   $ rippy .
   ```
2. **Search for a pattern in files and display as relative paths**:
   ```sh
   $ rippy . "TODO" -p 
   ```
3. **Search for a regex pattern in only `.txt` files and sort results by size**:
   ```sh
   $ rippy /path/to/docs "\b[A-z]{3}\b" -X "*.txt" -s --sort size
   ```   
5. **Limit search depth and sort by last modified date**:
   ```sh
   $ rippy /home/user -d -L 3 --sort date
   ```
5. **Include hidden files but ignore `.git` directory and export results to JSON**:
   ```sh
   $ rippy ./ --all -I ".git" -o "results.json"
   ```

### Detailed Overview
The most basic `rippy` operation is to simply output the contents of a provided directory as a tree in the terminal:

```shell
$ rippy "../rippy"

 ../rippy
  ├── Cargo.lock 
  ├── Cargo.toml
  ├── README.MD
  ╰── src
      ├── args.rs
      ├── dir.rs
      ├── main.rs
      ├── tcolor.rs
      ╰── tree.rs

1 directory, 8 files
```

Leveraging rayon to execute fast multi-threaded file operations, `rippy` offers extensive functionality, including the ability to open and search the contents of files, traverse directories, and apply parameters to ignore or target specific patterns or extensions. Combining the beauty of the Unix `tree` command with the usefulness of `grep`, `rippy` can search file contents and display the context around matched snippets. For example, you can search only the `*.rs` files in the project source directory for mentions of the substring `'JSON'` and export the results:

```shell
$ rippy ../rippy -sd "JSON" -X "*.rs" -o "json-mentioned.json"

 ../rippy
  ╰── src 
      ├── (2024-07-25 16:06:05,  12 K) args.rs ...Export the results as JSON to specified file...
      ├── (2024-07-24 21:47:55, 4.6 K) main.rs ...// Output tree as JSON to file provided
      ╰── (2024-07-25 17:44:28,  17 K) tree.rs ...the Tree structure to JSON and writes it to...

3 matches, 5 searched
```

The flags `-sd` specify additional display options, `-s` or `--size` will display the file size, and `-d` or `--date` displays the timestamp the file was last modified. The `-X` or `--include` argument is used to only search files matching the provided pattern, in this case any file that ends with a `.rs` extension. The `-O` or `--output` argument specifies an output filename to export any results to. Finally, any second positional argument will be treated as the pattern to search the file contents for, in this the substring `JSON`. The output contents in `json-mentioned.json` will be formatted as valid `JSON` objects:

```json
{
  "name": "../rippy",
  "entry_type": "Directory",
  "full_path": "../rippy",
  "last_modified": "2024-07-25 17:55:35",
  "size": 33737,
  "window": null,
  "children": [
    {
      "name": "src",
      "entry_type": "Directory",
      "full_path": "../rippy/src",
      "last_modified": "2024-07-24 17:11:53",
      "size": 33737,
      "window": null,
      "children": [
        {
          "name": "args.rs",
          "entry_type": "File",
          "full_path": "../rippy/src/args.rs",
          "last_modified": "2024-07-25 16:06:05",
          "size": 12180,
          "window": "...Export the results as JSON to specified file...",
          "children": []
        },
        {
          "name": "tree.rs",
          "entry_type": "File",
          "full_path": "../rippy/src/tree.rs",
          "last_modified": "2024-07-25 17:44:28",
          "size": 16973,
          "window": "...the Tree structure to JSON and writes it to...",
          "children": []
        },
        {
          "name": "main.rs",
          "entry_type": "File",
          "full_path": "../rippy/src/main.rs",
          "last_modified": "2024-07-24 21:47:55",
          "size": 4584,
          "window": "...// Output tree as JSON to file provided",
          "children": []
        }
      ]
    }
  ]
}
```

Results can be displayed and sorted using various options and keys. Here we search the project directory for any mentions of the string "registry" including only those files startng with the characters "win", excluding the "build" directory and sort the results by file size in ascending order. A snippet of the matched contents will be included by default but can be omitted by passing the `-w` or `windowless` option. If the default snippet size is too narrow or too wide, a custom character radius can be set with `-r <WIDTH>` or `--window-radius <WIDTH>`:

```shell
$ ./rippy "../rippy" -sd --sort "size" -I "build" -X "win*" -r 26 "registry"

 ../rippy
  ╰── target
      ╰── debug
          ╰── deps
              ├── (2024-07-29 18:12:54, 817 B) windows_x86_64_msvc-4d3fbecefb660e4c.d ...C:\Users\.cargo\registry\src\index.crates.io-6f17d...
              ├── (2024-07-29 18:12:19, 1.8 K) winapi_util-6e8e4993ad95c0bf.d         ...C:\Users\.cargo\registry\src\index.crates.io-6f17d...
              ├── (2024-07-29 18:12:59, 5.7 K) windows_sys-42f4f44881857ce7.d         ...C:\Users\.cargo\registry\src\index.crates.io-6f17d...
              ╰── (2024-07-29 18:12:56,  15 K) winapi-a718b57d4cfcbb07.d              ...C:\Users\.cargo\registry\src\index.crates.io-6f17d...

4 matches, 10 searched
```

Results can be sorted 8 different ways using four different keys:
- `date`: Sorts by last modified date from oldest to most recent.
- `name`: Sorts alphabetically by entry name in case-sensitive fashion.
- `size`: Sorts by content size in bytes from smallest to largest.
- `type`: Sorts by entry type from directories first to files last.

To reverse the sorting order for a given key, the `-z` or `--reverse` flag can be provided. When no sort key is specified, `name` will be used. When no ordering is specified, ascending order will be used by default.

## License
This project is [MIT licensed](https://github.com/AnteT/rippy/blob/master/LICENSE) with the source code for `rippy`, currently a work in progress, [openly available](https://github.com/AnteT/rippy).

Thank you!

_- Ante Tonkovic-Capin_
