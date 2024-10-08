// #![allow(unused)]
use std::hash::BuildHasherDefault;
use std::fmt;
use std::fs;
// use std::io::IsTerminal;
use std::path;
use std::io::{self, Write, stdout};
use std::path::PathBuf;
use std::time::{UNIX_EPOCH, Duration};

use crate::args::RippyArgs;
use crate::{ansi_color, concat_str};
use crate::dir::TreeLeaf;

use chrono;
use is_executable::is_executable;
use regex::Regex;
use serde::{Serialize, Deserialize}; // Add Serialize and Deserialize traits
use serde_json::json;
use indexmap::IndexMap; // Ordered map
use ahash::AHasher; // Faster hashing

type TreeMap<K, V> = IndexMap<K, V, BuildHasherDefault<AHasher>>; // TreeMap type alias

/// Units to scale size value accordingly
const KB:f64 = 1_000.0;
const MB:f64 = 1_000_000.0;
const GB:f64 = 1_000_000_000.0;

/// Global left margin for entire single space tree offset. 
const MARGIN_LEFT: &'static str = "\u{0020}";

/// Non-breaking single space for output com­pat­i­bil­i­ty with UNIX `tree` command
const NB_SINGLE: &'static str = "\u{00A0}";

/// Enum to differentiate between Directory and File type objects in Tree struct.
#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize, PartialOrd, Ord)] // Derive Serialize and Deserialize
pub enum EntryType {
    Directory,
    File,
}

// Implement Display for EntryType to convert to string
impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Primary struct for tree module, providing methods for core functionality.
#[derive(Clone, Serialize, Deserialize)] // Derive Serialize and Deserialize
pub struct Tree {
    pub display: String,
    pub name: String,
    pub path: Option<PathBuf>,
    pub entry_type: EntryType,
    pub last_modified: Option<f64>,
    pub size: Option<u64>,
    pub window: Option<String>,
    pub fmt_width: Option<usize>,
    pub children: TreeMap<String, Tree>,
}

impl From<TreeLeaf> for Tree {
    /// Converts a TreeLeaf into a Tree by consuming the original and avoiding redundant or unnecessary allocations during the processs.
    fn from(value: TreeLeaf) -> Self {
        let (entry_type, path, fmt_width, window) = if value.is_dir {
            (EntryType::Directory, None, None, None)
        } else {
            (EntryType::File, if !value.is_sym { Some(PathBuf::from(value.relative_path)) } else { None }, None, value.window)
        };
        Tree::new(value.display, value.name, path, entry_type, value.last_modified, value.size, fmt_width, window)
    }
}
impl Tree {
    /// Creates a new tree using a root path and TreeMap for children nodes
    pub fn new(display: impl Into<String>, name: impl Into<String>, path: Option<PathBuf>, entry_type: EntryType, last_modified: Option<f64>, size: Option<u64>, fmt_width: Option<usize>, window: Option<String>) -> Self {
        Tree {
            display: display.into(),
            name: name.into(),
            path,
            entry_type,
            last_modified,
            size,
            fmt_width,
            window,
            children: TreeMap::default(),
        }
    }

    /// REVISED: Creates a new `Tree` given a path explicitely for creating missing `Directory` components. Assumes path given is already standardized to contain forward slashes only.
    pub fn from_dir(path: std::path::PathBuf, args: &RippyArgs) -> Self {
        let name = path.file_name().map_or_else(|| path.to_string_lossy().to_string(), |p| p.to_string_lossy().to_string());
        let display = if args.show_relative_path {
            path.to_string_lossy().to_string()
        } else if args.show_full_path {
            convert_relative_to_abs_path(&path.to_string_lossy().to_string())
        } else {
            name.clone()
        };
        let display = if args.is_quote { concat_str!("\"", display, "\"") } else { display };        
        let entry_type = EntryType::Directory;
        let (last_modified, size) = if args.show_size || args.show_date {
            let metadata = fs::metadata(&path).ok();
            let last_modified = if args.show_date { convert_metadata_to_f64(&metadata) } else { None };
            let size = if args.show_size { metadata.as_ref().map(|meta| meta.len()) } else { None };
            (last_modified, size)
        } else {
            (None, None)
        };
        let (fmt_width, window, children) = (None, None, TreeMap::default());
        Tree { display, name, path: None, entry_type, last_modified, size, fmt_width, window, children }
    }

    /// Recursively calculates the size of directories based on their children
    pub fn calculate_sizes(&mut self) {
        if self.entry_type == EntryType::Directory {
            let mut total_size = 0;

            for child in self.children.values_mut() {
                child.calculate_sizes();
                if let Some(size) = child.size {
                    total_size += size;
                }
            }
            self.size = Some(total_size);
        }
    }

    /// Calculates the max file name length for all the files in a single directory and assigns that value to the self.fmt_width property for the directory and its children.
    pub fn calculate_fmt_width(&mut self) {
        if self.entry_type == EntryType::Directory {
            let mut max_length = 0;

            // Find the max file name length in the current directory
            for child in self.children.values() {
                let name_length = child.display.len();
                if name_length > max_length {
                    max_length = name_length;
                }
            }

            self.fmt_width = Some(max_length as usize);

            // Set fmt_width for all children in the current directory
            for child in self.children.values_mut() {
                if child.entry_type == EntryType::File {
                    child.fmt_width = Some(max_length as usize);
                } else if child.entry_type == EntryType::Directory {
                    child.fmt_width = Some(max_length as usize);
                    // Recursively calculate and set fmt_width for the child directory
                    child.calculate_fmt_width();
                }
            }
        }
    }

    /// LEGACY: Recursively prints the tree structure tied to the `Tree` instance directly as an uncolored legacy version compatible with `tree` output.
    /// For example, using a valid object of type `Tree`, call with:
    /// 
    ///     tree.print_legacy("", 0, true);
    /// 
    /// Which will render output that can be diff'd against the unix tree command:
    /// 
    /// ```shell
    /// .
    /// ├── Cargo.lock
    /// ├── Cargo.toml
    /// ├── README.MD
    /// └── src
    ///     ├── args.rs
    ///     ├── main.rs
    ///     └── tree.rs
    /// 
    /// 1 directory, 6 files
    /// ```
    #[allow(unused)]
    pub fn print_legacy(&mut self, indent: &str, depth: usize, is_last: bool) {
        let connector = if depth == 0 {
            ""
        } else if is_last {
            "└── "
        } else {
            "├── "
        };

        println!("{}{}{}", indent, connector, self.display);

        let new_indent = if depth == 0 {
            if is_last { indent } else { &concat_str!(indent, "│") }
        } else {
            if is_last { &concat_str!(indent, "    ") } else { &concat_str!(indent, "│", NB_SINGLE, NB_SINGLE, " ") }
        };
        
        // Sort entries by name as default until `args` brought into scope through function to get comparator
        self.children.sort_by(|_, a, _, b| a.display.cmp(&b.display));
        let child_count = self.children.len().saturating_sub(1);

        for (i, child) in self.children.values_mut().enumerate() {
            child.print_legacy(&new_indent, depth + 1, i == child_count);
        }

        // Newline when tree is finished
        if depth == 0 {
            println!("");
        }
    }

    /// Converts the Tree structure to JSON and writes it to a file
    pub fn write_to_json_file(&self, settings: &RippyArgs) -> std::io::Result<()> {
        // Harmonize into expected generic type
        let file_path = &settings.output;

        // Use a closure to capture `settings`
        let convert_children = |children: &TreeMap<String, Tree>| {
            children.values().map(|child| child.to_json(settings)).collect::<Vec<serde_json::Value>>()
        };

        // Construct the json
        let json_value = json!({
            "name": self.name,
            "entry_type": self.entry_type.to_string(),
            "last_modified": format_json_datetime(self.last_modified),
            "size": self.size,
            "window": format_json_window(&self.window),
            "children": convert_children(&self.children),
        });

        // Open the file and wrap it in BufWriter for efficient writing
        let file = std::fs::File::create(file_path)?;
        let buf_wrtier = io::BufWriter::new(file);

        serde_json::to_writer_pretty(buf_wrtier, &json_value)?;

        Ok(())
    }

    /// Converts the Tree structure to JSON Value
    pub fn to_json(&self, settings: &RippyArgs) -> serde_json::Value {
        let convert_children = |children: &TreeMap<String, Tree>| {
            children.values().map(|child| child.to_json(settings)).collect::<Vec<serde_json::Value>>()
        };
        json!({
            "name": self.name,
            "entry_type": self.entry_type.to_string(),
            "last_modified": format_json_datetime(self.last_modified),
            "size": self.size,
            "window": format_json_window(&self.window),
            "children": convert_children(&self.children),
        })
    }

    /// Tree for root with specific considerations for rendering and pathing traversal to facilitate construction and building. Expected display field assigned to name for both name and relative path option, using full path when canonical argument is present.
    pub fn new_root(root:&std::path::PathBuf, args: &RippyArgs) -> Self {
        // No distinction is made between show_relative_path or not for root of tree, only if full path needed is relevant as root name will be used for building/traversal
        let root_name = if !args.show_full_path {
            root.to_string_lossy().to_string()
        } else {
            convert_relative_to_abs_path(&root.to_string_lossy().to_string())
        };
        let name = root_name.clone();
        let root_name = if args.is_quote { concat_str!("\"", root_name, "\"") } else { root_name };
        // Create root of tree from directory provided in initial args and a relative path with "/" suffix that can be used for traversal and component building.
        Tree::new( root_name, name, None, EntryType::Directory, None, None, None, None )
    }
}

impl std::fmt::Debug for Tree {
    /// Display the directory structure and any children of directory entries
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tree({} [{}, {} Children])", self.display, self.entry_type, self.children.len())?;
        Ok(())
    }
}

impl fmt::Display for Tree {
    /// Display the directory structure and any children of directory entries
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Name: {} <{:?}>", self.display, self.entry_type)?;
        if self.children.len() >= 1 {
            writeln!(f, "Children:")?;
            for (name, child) in &self.children {
                writeln!(f, "  {} <{:?}>", name, child.entry_type)?;
            }
        }
        Ok(())
    }
}

/// Tracks resulting file and directory counts for summary outputs.
#[derive(Debug, PartialEq, Eq)]
pub struct TreeCounts {
    pub dir_count: usize,
    pub file_count: usize
}

impl TreeCounts {
    pub fn new() -> Self {
        TreeCounts {
            dir_count: 0,
            file_count: 0
        }
    }
}

/// Extracts the SystemTime from the fs::Metadata and converts to f64 seconds duration since unix epoch.
fn convert_metadata_to_f64(metadata: &Option<fs::Metadata>) -> Option<f64> {
    metadata
        .as_ref() // Convert from Option<Metadata> to Option<&Metadata>
        .and_then(|meta| meta.modified().ok()) // Get the SystemTime and handle errors
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok()) // Get the duration since the UNIX_EPOCH
        .map(|duration| duration.as_secs_f64()) // Convert the duration to f64
}

/// Formats the seconds since unix epoch as a ISO-8601 tz naive timestamp regardless of settings specifically for JSON export.
fn format_json_datetime(last_modified: Option<f64>) -> Option<String> {
    let dt_format = "%Y-%m-%d %H:%M:%S";
        last_modified.map_or_else(|| None, |timestamp| {
            // Convert f64 to Duration
            let duration_since_epoch = Duration::from_secs_f64(timestamp);
            let datetime = chrono::DateTime::from_timestamp(duration_since_epoch.as_secs() as i64, duration_since_epoch.subsec_nanos()).unwrap_or_default();
            Some(datetime.format(dt_format).to_string())
        })
    }

/// Formats the seconds since unix epoch as a human readable timestamp based on the provided settings and EntryType.
fn format_display_datetime(last_modified: Option<f64>, settings: &RippyArgs, entry_type: EntryType) -> String {
    if settings.show_date {
        if !settings.is_dir_detail && entry_type == EntryType::Directory {
            return "".to_string();
        }
        let dt_format = if settings.is_short_date {"%Y-%m-%d"} else {"%Y-%m-%d %H:%M:%S"}; // "%Y-%m-%d %H:%M:%S" for [2024-07-24 15:09:57] or "%d-%b-%y" for [12-Jul-24]
        last_modified.map(|timestamp| {
            // Convert f64 to Duration
            let duration_since_epoch = Duration::from_secs_f64(timestamp);
            let datetime = chrono::DateTime::from_timestamp(duration_since_epoch.as_secs() as i64, duration_since_epoch.subsec_nanos()).unwrap_or_default();
            datetime.format(dt_format).to_string()
        }).unwrap_or_default()
    } else {
        "".to_string()
    }
}

/// Formats the window context for JSON export by removing all ANSI control and command sequences that may have been used for displaying the results in the tree
fn format_json_window(input: &Option<String>) -> Option<String> {
    let ansi_escape = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    input.as_deref().map(|x| ansi_escape.replace_all(&x, "").to_string())
}

/// Formats size according to scale using appropriate units to fit within fixed width to retain alignment when included in display. 
fn format_size(size:u64) -> String {
    // Convert size to f64
    let size = size as f64;

    if size < KB {
        // No conversion, already in bytes
        let size_in_unit = size;
        let size_as_str = if size_in_unit < 10.0 {format!("{:.1}", size_in_unit)} else {format!("{:.0}", size_in_unit)};
        concat_str!(format!("{:>3.3}", size_as_str), " B")
    } else if size < MB {
        // Convert to kilobytes
        let size_in_unit = size / KB;
        let size_as_str = if size_in_unit < 10.0 {format!("{:.1}", size_in_unit)} else {format!("{:.0}", size_in_unit)};
        concat_str!(format!("{:>3.3}", size_as_str), " K")
    } else if size < GB {
        // Convert to megabytes
        let size_in_unit = size / MB;
        let size_as_str = if size_in_unit < 10.0 {format!("{:.1}", size_in_unit)} else {format!("{:.0}", size_in_unit)};
        concat_str!(format!("{:>3.3}", size_as_str), " M")
    } else {
        // Convert to gigabytes
        let size_in_unit = size / GB;
        let size_as_str = if size_in_unit < 10.0 {format!("{:.1}", size_in_unit)} else {format!("{:.0}", size_in_unit)};
        concat_str!(format!("{:>3.3}", size_as_str), " G")
    }
}

/// Formats the display size based on the provided settings and entry type
fn format_display_size(size: Option<u64>, settings: &RippyArgs, entry_type: EntryType) -> String {
    if settings.show_size {
        if settings.is_dir_detail || entry_type == EntryType::File {
            size.map_or(String::new(), |s| format_size(s))
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    }
}

pub fn _tree_peek(paths: &Vec<(String, Option<String>)>) {
        for (path, _window) in paths {
            println!("{}", path);
        }
    }

/// Converts relative path to full canonical path replacing any backslashes with forward slashes to display with `Tree` results if needed.
fn convert_relative_to_abs_path(relative_path: &str) -> String {
    path::absolute(path::Path::new(relative_path)).map_or(relative_path.to_owned(), |path| path.to_string_lossy().replace("\\","/"))
}

/// Optimized version to build the `Tree` structure given an owned set of `TreeLeafs` to iteratively build from.
pub fn build_tree_from_paths(paths: Vec<TreeLeaf>, args: &'static RippyArgs) -> Tree {
    // Create root of tree from directory provided in initial args
    let mut root_tree = Tree::new_root(&args.directory, &args);

    let root_path = args.directory.to_string_lossy().to_string();
    let root_path_length = root_path.len();
    let root_standard_path = if !root_path.ends_with("/") {
        concat_str!(root_path, "/")
    } else {
        root_path.to_owned()
    };

    // Traverse each leaf and build the tree
    let mut last_parent = "".to_string();
    let mut current_dir = &mut root_tree;

    for leaf in paths.into_iter() {
        // Compute relative path to avoid unnecessary allocations
        let traversal_path = if leaf.relative_path.starts_with(&root_path) { &leaf.relative_path[root_path_length..] } else { &leaf.relative_path };
        let leaf_components: Vec<&str> = traversal_path.split('/').filter(|s| !s.is_empty()).collect();
        let leaf_components = if let Some((_, c)) = leaf_components.split_last() { c } else { &leaf_components };
        let current_parent = if let Some(&p) = leaf_components.last() { p.to_string() } else { "".to_string() };

        // Quick insertion of node in scenario where parent is the same as last iteration to avoid wasting time iterating to required depth
        if last_parent == current_parent {
            current_dir.children.insert(leaf.name.clone(), leaf.into());
            continue;
        } else {
            // Update current directory reference by reseting to root
            current_dir = &mut root_tree;
    
            for (pid, parent) in leaf_components.iter().enumerate() {
                let entry = current_dir.children.entry(parent.to_string());
                current_dir = entry.or_insert_with(|| {
                    let current_path_state = concat_str!(root_standard_path, &leaf_components[0..=pid].join("/"));
                    Tree::from_dir(std::path::PathBuf::from(current_path_state), &args)
                });
            }
            // Insert the leaf
            last_parent = current_parent; // Update last_parent for next iteration
            current_dir.children.insert(leaf.name.clone(), leaf.into());
        }
    }
    root_tree
}

/// Returns the number of digits in the provided value using a more performant log based approach.
fn count_digits_log(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    // Calculate the number of digits
    ((n as f64).log(10.0).floor() as usize) + 1
}

/// REVISED WITHOUT COLOR CHECK: Creates the graphical terminal representation of the tree by iteratively printing the tree line by line using specified settings with active TTY check for ANSI coloring.
fn write_tree_to_buf(tree: &mut Tree, enumeration: &str, depth: u32, prefix: &str, is_last: bool, args: &RippyArgs, counts: &mut TreeCounts, writer: &mut impl Write) -> io::Result<()> {
    // Establish display name format
    let display_name = &tree.display;
    // Handle optional display time or date last modified of contents
    let display_datetime = format_display_datetime(tree.last_modified, args, tree.entry_type);
    // Handle optional display size
    let display_size = format_display_size(tree.size, args, tree.entry_type);
    // Handle details for how to display both size and date if applicable
    let file_date_size_details = match (display_datetime.is_empty(), display_size.is_empty()) {
        (true, true) => "".to_string(),
        (true, false) | (false, true) => concat_str!("(", display_datetime, display_size, ") "),
        (false, false) => concat_str!("(", display_datetime, ", ", display_size, ") "),
    };

    if depth == 0 {
        let root_name = ansi_color!(&args.colors.root, bold=!args.is_grayscale, display_name);
        writeln!(writer, "{}", concat_str!(MARGIN_LEFT, &root_name))?;
    } else {
        // Count dirs and files and determine styling
        let (color, time_color, is_bold, padding) = match tree.entry_type {
            EntryType::Directory => {
                counts.dir_count += 1;
                (
                    &args.colors.dir,
                    &args.colors.detail,
                    !args.is_grayscale,
                    "".to_string(), // Return a &str
                )
            },
            EntryType::File => {
                counts.file_count += 1;
                let window_padding = if args.is_search && args.is_window {tree.fmt_width.map(|w| " ".repeat(w - &tree.display.len() + 1)).unwrap_or_else(|| "".to_string())} else {"".to_string()};
                (
                    // Don't worry about color if its grayscale or if the path is None or then finally if the path is not executable
                    if args.is_grayscale || tree.path.is_none() {&None} else { if tree.path.as_ref().map_or_else(|| true, |p| !is_executable(p))  {&args.colors.file} else {&args.colors.exec}},
                    // if args.is_grayscale || tree.path.as_ref().map_or_else(|| true, |p| !is_executable(p)) { &args.colors.file } else { &args.colors.exec },
                    &args.colors.detail,
                    false,
                    window_padding,
                )
            },
        };
        // Style the connector based on the depth
        let connector_color = if depth == 1 {
            &args.colors.root
        } else {
            &args.colors.dir
        };
        let indent_bar = "─".repeat(args.indent) + " ";
        let connector = if args.is_flat {
            "".to_string()
        } else if is_last {
            ansi_color!(connector_color, bold=false, concat_str!("╰", indent_bar))
        } else {
            ansi_color!(connector_color, bold=false, concat_str!("├", indent_bar))
        };

        // Enumeration prefix
        let enum_prefix: String = if args.is_enumerate && depth != 0 {
            ansi_color!(args.colors.detail, bold=false, concat_str!("[", enumeration, "] "))
        } else {
            "".to_string()
        };

        let entry_name = ansi_color!(color,bold=is_bold, display_name);
        let entry_details = if file_date_size_details.is_empty() { file_date_size_details } else { ansi_color!(time_color, bold=false, file_date_size_details) };
        let entry_window = tree.window.as_ref().map_or("", |p| p);
        writeln!(writer, "{}", concat_str!(MARGIN_LEFT,prefix,connector,enum_prefix,entry_details,entry_name,padding,entry_window))?;
    }

    let level_indent = NB_SINGLE.repeat(args.indent) + " ";
    let new_prefix = if args.is_flat {
        "".to_string()
    } else if depth == 0 {
        prefix.to_string()
    } else if is_last {
        concat_str!(prefix, level_indent, " ")
    } else {
        let pipe_color = if depth == 1 {
            &args.colors.root
        } else {
            &args.colors.dir
        };
        concat_str!(prefix, ansi_color!(pipe_color, bold=false, "│"), level_indent)
    };

    // Collect children into a single vector and sort according to args
    tree.children.sort_by(|_, a, _, b| (args.sort_by)(a, b));

    // Determine the count of files for truncation
    let total_files = tree.children.values().into_iter().filter(|c| c.entry_type == EntryType::File).count();

    // Truncate the list if necessary
    if total_files > args.max_files {
        let mut files_seen = 0;
        tree.children.retain(|_, child| {
            if child.entry_type == EntryType::File {
                if files_seen < args.max_files {
                    files_seen += 1;
                    true
                } else {
                    false
                }
            } else {
                true
            }
        });

        // Add a truncation entry if necessary and count files truncated
        if files_seen >= args.max_files {
            let trunc_num = total_files - args.max_files;
            counts.file_count += trunc_num - 1;
            let trunc_fmt = concat_str!(trunc_num.to_string(), " more ...");
            let trunc_label = ansi_color!(&args.colors.detail, bold=false, trunc_fmt);
            tree.children.insert(trunc_label.to_owned(), Tree::new(&trunc_label, &trunc_label, None, EntryType::File, None, None, None, None));
        }
    }

    // Print each child
    let last_index = tree.children.len().saturating_sub(1);
    for (i, child) in tree.children.values_mut().enumerate() {
        let is_last_child = i == last_index;
        // Enumeration padding if needed
        let enumeration = if args.is_enumerate {
            let enum_padding = count_digits_log(last_index.saturating_add(1)).saturating_sub(count_digits_log(i.saturating_add(1)));
            &concat_str!(" ".repeat(enum_padding), i.saturating_add(1).to_string())
        } else { "" };

        write_tree_to_buf(child, enumeration, depth + 1, &new_prefix, is_last_child, args, counts, writer)?;
    }

    if depth == 1 && is_last {
        writeln!(writer)?;
    }

    Ok(())
}

/// Wrapper to handle printing of tree without coloring main with result.
pub fn print_tree(tree: &mut Tree, args: &RippyArgs, counts: &mut TreeCounts) -> io::Result<()> {
    let stdout = stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    write_tree_to_buf(tree, "", 0, "", true, &args, counts, &mut writer)
}

/// Traverses the tree to return the appropriate counts of each type of entry, ignoring the initial root directory target of the search.
pub fn count_tree(tree: &Tree, counts: &mut TreeCounts, is_first: bool) {
    match tree.entry_type {
        EntryType::Directory => {if !is_first {counts.dir_count += 1;}},
        EntryType::File => counts.file_count += 1,
    }
    for child in tree.children.values() {
        count_tree(child, counts, false);
    }
}