use std::fmt;
use std::fs;
use std::hash::BuildHasherDefault;
use std::io::{self, stdout, Write};
use std::path::{self, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, UNIX_EPOCH};

use ahash::AHasher;
use indexmap::IndexMap;
use is_executable::is_executable;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::args::RippyArgs;
use crate::crawl::{MatchWindow, TreeLeaf};
use crate::{ansi_color, concat_str};

pub type TreeMap = IndexMap<String, Tree, BuildHasherDefault<AHasher>>;

const KB: f64 = 1_000.0;
const MB: f64 = 1_000_000.0;
const GB: f64 = 1_000_000_000.0;
const MARGIN_LEFT: &str = "\u{0020}";
const NB_SINGLE: &str = "\u{00A0}";
const SOLID_VERTICAL: &str = "\u{2502}";
const LAST_CONNECTOR: &str = "\u{2570}";
const MID_CONNECTOR: &str = "\u{251C}";
const HORIZONTAL: &str = "\u{2500}";

/*
Quick separator toggle for search results based on which one I think looks best, still haven't decided 100 percent:

" | "  // default: subtle visual divider
" │ "  // full length vertical divider (connects to next one above or below if present): hard visual divider
" ┆ "  // lighter dotted divider
" -> " // explicit arrow
" :: " // plain text divider
" — "  // minimal dash
"   "  // effectively off, keeps spacing only

*/
const SEARCH_SNIPPET_SEPARATOR: &str = " ┆ ";
// const SEARCH_SNIPPET_SEPARATOR: &str = " ╎ "; // also doesn't look too bad, but less homogenous across multiple match snippets

static ANSI_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap());

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize, PartialOrd, Ord)]
pub enum EntryType {
    Directory,
    File,
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct TreeIter<'a> {
    stack: Vec<&'a Tree>,
}

impl<'a> Iterator for TreeIter<'a> {
    type Item = &'a Tree;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.stack.pop() {
            for child in current.children.values().rev() {
                self.stack.push(child);
            }
            Some(current)
        } else {
            None
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Tree {
    pub display: String,
    pub name: String,
    pub path: Option<PathBuf>,
    pub entry_type: EntryType,
    pub last_modified: Option<f64>,
    pub size: Option<u64>,
    pub windows: Vec<MatchWindow>,
    pub fmt_width: Option<usize>,
    pub children: TreeMap,
}

impl From<TreeLeaf> for Tree {
    fn from(value: TreeLeaf) -> Self {
        let (entry_type, path, fmt_width, windows) = if value.is_dir {
            (EntryType::Directory, None, None, Vec::new())
        } else {
            (
                EntryType::File,
                if !value.is_sym {
                    Some(PathBuf::from(value.relative_path))
                } else {
                    None
                },
                None,
                value.windows,
            )
        };

        Tree::new(
            value.display,
            value.name,
            path,
            entry_type,
            value.last_modified,
            value.size,
            fmt_width,
            windows,
        )
    }
}

impl Tree {
    pub fn new(
        display: impl Into<String>,
        name: impl Into<String>,
        path: Option<PathBuf>,
        entry_type: EntryType,
        last_modified: Option<f64>,
        size: Option<u64>,
        fmt_width: Option<usize>,
        windows: Vec<MatchWindow>,
    ) -> Self {
        Tree {
            display: display.into(),
            name: name.into(),
            path,
            entry_type,
            last_modified,
            size,
            windows,
            fmt_width,
            children: TreeMap::default(),
        }
    }

    pub fn from_dir(path: PathBuf, args: &RippyArgs) -> Self {
        let name = path
            .file_name()
            .map_or_else(|| path.to_string_lossy().to_string(), |value| value.to_string_lossy().to_string());
        let display = if args.show_relative_path {
            path.to_string_lossy().to_string()
        } else if args.show_full_path {
            convert_relative_to_abs_path(&path.to_string_lossy())
        } else {
            name.clone()
        };
        let display = if args.is_quote {
            concat_str!("\"", display, "\"")
        } else {
            display
        };
        let entry_type = EntryType::Directory;
        let metadata = if args.show_size || args.show_date {
            fs::metadata(&path).ok()
        } else {
            None
        };
        let last_modified = if args.show_date {
            convert_metadata_to_f64(&metadata)
        } else {
            None
        };
        let size = if args.show_size {
            metadata.as_ref().map(|meta| meta.len())
        } else {
            None
        };
        Tree {
            display,
            name,
            path: None,
            entry_type,
            last_modified,
            size,
            windows: Vec::new(),
            fmt_width: None,
            children: TreeMap::default(),
        }
    }

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

    pub fn calculate_fmt_width(&mut self, args: &RippyArgs) {
        if self.entry_type != EntryType::Directory {
            return;
        }

        let align_across_siblings = args.max_display_matches <= 1;

        if align_across_siblings {
            let dir_max_width = self
                .children
                .values()
                .filter(|child| child.entry_type == EntryType::File)
                .map(|child| {
                    child.windows
                        .first()
                        .map(|window| visible_width(&build_plain_path_line_label(&child.display, window.line_number)))
                        .unwrap_or_else(|| visible_width(&child.display))
                })
                .max()
                .unwrap_or(0);

            for child in self.children.values_mut() {
                if child.entry_type == EntryType::File {
                    child.fmt_width = Some(dir_max_width);
                } else {
                    child.calculate_fmt_width(args);
                }
            }
        } else {
            for child in self.children.values_mut() {
                if child.entry_type == EntryType::File {
                    let max_label_width = child
                        .windows
                        .iter()
                        .map(|window| visible_width(&build_plain_path_line_label(&child.display, window.line_number)))
                        .max()
                        .unwrap_or_else(|| visible_width(&child.display));

                    child.fmt_width = Some(max_label_width);
                } else {
                    child.calculate_fmt_width(args);
                }
            }
        }
    }

    pub fn write_to_json_file(&self, settings: &RippyArgs) -> io::Result<()> {
        let file = fs::File::create(&settings.output)?;
        let writer = io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.to_json(settings))?;
        Ok(())
    }

    pub fn to_json(&self, settings: &RippyArgs) -> serde_json::Value {
        let children = self
            .children
            .values()
            .map(|child| child.to_json(settings))
            .collect::<Vec<_>>();
        json!({
            "name": self.name,
            "entry_type": self.entry_type.to_string(),
            "last_modified": format_json_datetime(self.last_modified),
            "size": self.size,
            "window": format_json_first_window(&self.windows),
            "windows": format_json_windows(&self.windows),
            "children": children,
        })
    }

    pub fn new_root(root: &PathBuf, args: &RippyArgs) -> Self {
        let root_name = if !args.show_full_path {
            root.to_string_lossy().to_string()
        } else {
            convert_relative_to_abs_path(&root.to_string_lossy())
        };
        let name = root_name.clone();
        let root_name = if args.is_quote {
            concat_str!("\"", root_name, "\"")
        } else {
            root_name
        };
        Tree::new(root_name, name, None, EntryType::Directory, None, None, None, Vec::new())
    }

    pub fn iter(&self) -> TreeIter<'_> {
        TreeIter { stack: vec![self] }
    }
}

impl std::fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Tree({} [{}, {} Children])",
            self.display,
            self.entry_type,
            self.children.len()
        )
    }
}

impl fmt::Display for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Name: {} <{:?}>", self.display, self.entry_type)?;
        if !self.children.is_empty() {
            writeln!(f, "Children:")?;
            for (name, child) in &self.children {
                writeln!(f, "  {} <{:?}>", name, child.entry_type)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TreeCounts {
    pub dir_count: usize,
    pub file_count: usize,
}

impl TreeCounts {
    pub fn new() -> Self {
        Self {
            dir_count: 0,
            file_count: 0,
        }
    }
}

fn convert_metadata_to_f64(metadata: &Option<fs::Metadata>) -> Option<f64> {
    metadata
        .as_ref()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs_f64())
}

fn format_json_datetime(last_modified: Option<f64>) -> Option<String> {
    let dt_format = "%Y-%m-%d %H:%M:%S";
    last_modified.and_then(|timestamp| {
        let duration_since_epoch = Duration::from_secs_f64(timestamp);
        chrono::DateTime::from_timestamp(
            duration_since_epoch.as_secs() as i64,
            duration_since_epoch.subsec_nanos(),
        )
        .map(|datetime| datetime.format(dt_format).to_string())
    })
}

fn format_display_datetime(last_modified: Option<f64>, settings: &RippyArgs, entry_type: EntryType) -> String {
    if !settings.show_date {
        return String::new();
    }

    if !settings.is_dir_detail && entry_type == EntryType::Directory {
        return String::new();
    }

    let dt_format = &settings.date_format;
    last_modified
        .and_then(|timestamp| {
            let duration_since_epoch = Duration::from_secs_f64(timestamp);
            chrono::DateTime::from_timestamp(
                duration_since_epoch.as_secs() as i64,
                duration_since_epoch.subsec_nanos(),
            )
            .map(|datetime| datetime.format(dt_format).to_string())
        })
        .unwrap_or_default()
}

fn format_json_first_window(windows: &[MatchWindow]) -> Option<String> {
    windows.first().map(format_json_window)
}

fn format_json_windows(windows: &[MatchWindow]) -> Vec<String> {
    windows.iter().map(format_json_window).collect()
}

fn format_json_window(window: &MatchWindow) -> String {
    match window.line_number {
        Some(line_number) if !window.text.is_empty() => {
            let prefix = line_number.to_string();
            let cleaned = strip_ansi(&window.text);
            concat_str!(prefix, " ", cleaned)
        }
        Some(line_number) => line_number.to_string(),
        None => strip_ansi(&window.text),
    }
}

fn format_size(size: u64) -> String {
    let size = size as f64;

    if size < KB {
        let size_as_str = if size < 10.0 {
            format!("{:.1}", size)
        } else {
            format!("{:.0}", size)
        };
        let padded = format!("{:>3.3}", size_as_str);
        concat_str!(padded, " B")
    } else if size < MB {
        let size_in_unit = size / KB;
        let size_as_str = if size_in_unit < 10.0 {
            format!("{:.1}", size_in_unit)
        } else {
            format!("{:.0}", size_in_unit)
        };
        let padded = format!("{:>3.3}", size_as_str);
        concat_str!(padded, " K")
    } else if size < GB {
        let size_in_unit = size / MB;
        let size_as_str = if size_in_unit < 10.0 {
            format!("{:.1}", size_in_unit)
        } else {
            format!("{:.0}", size_in_unit)
        };
        let padded = format!("{:>3.3}", size_as_str);
        concat_str!(padded, " M")
    } else {
        let size_in_unit = size / GB;
        let size_as_str = if size_in_unit < 10.0 {
            format!("{:.1}", size_in_unit)
        } else {
            format!("{:.0}", size_in_unit)
        };
        let padded = format!("{:>3.3}", size_as_str);
        concat_str!(padded, " G")
    }
}

fn format_display_size(size: Option<u64>, settings: &RippyArgs, entry_type: EntryType) -> String {
    if !settings.show_size {
        return String::new();
    }

    if settings.is_dir_detail || entry_type == EntryType::File {
        size.map_or_else(String::new, format_size)
    } else {
        String::new()
    }
}

pub fn _tree_peek(paths: &Vec<(String, Vec<MatchWindow>)>) {
    for (path, _windows) in paths {
        println!("{}", path);
    }
}

fn convert_relative_to_abs_path(relative_path: &str) -> String {
    path::absolute(path::Path::new(relative_path))
        .map_or_else(|_| relative_path.to_owned(), |path| path.to_string_lossy().replace('\\', "/"))
}

pub fn build_tree_from_paths(paths: Vec<TreeLeaf>, args: &RippyArgs) -> Tree {
    let mut root_tree = Tree::new_root(&args.directory, args);

    let root_path = args.directory.to_string_lossy().to_string();
    let root_path_length = root_path.len();
    let root_standard_path = if root_path.ends_with('/') {
        root_path.clone()
    } else {
        concat_str!(root_path, "/")
    };

    let mut last_parent = String::new();
    let mut current_dir = &mut root_tree;

    for leaf in paths {
        let traversal_path = if leaf.relative_path.starts_with(&root_path) {
            &leaf.relative_path[root_path_length..]
        } else {
            &leaf.relative_path
        };
        let leaf_components: Vec<&str> = traversal_path.split('/').filter(|segment| !segment.is_empty()).collect();
        let leaf_components = if let Some((_, components)) = leaf_components.split_last() {
            components
        } else {
            &leaf_components
        };
        let current_parent = leaf_components.join("/");

        if last_parent == current_parent {
            current_dir.children.insert(leaf.name.clone(), leaf.into());
            continue;
        }

        current_dir = &mut root_tree;
        for (pid, parent) in leaf_components.iter().enumerate() {
            let entry = current_dir.children.entry(parent.to_string());
            current_dir = entry.or_insert_with(|| {
                let joined_parent = leaf_components[0..=pid].join("/");
                let current_path_state = concat_str!(root_standard_path, &joined_parent);
                Tree::from_dir(PathBuf::from(current_path_state), args)
            });
        }

        last_parent = current_parent;
        current_dir.children.insert(leaf.name.clone(), leaf.into());
    }

    root_tree
}

fn count_digits_log(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        ((n as f64).log(10.0).floor() as usize) + 1
    }
}

pub fn write_tree_to_buf(
    tree: &mut Tree,
    enumeration: &str,
    depth: u32,
    prefix: &str,
    is_last: bool,
    args: &RippyArgs,
    counts: &mut TreeCounts,
    writer: &mut impl Write,
) -> io::Result<()> {
    let display_name = &tree.display;
    let display_datetime = format_display_datetime(tree.last_modified, args, tree.entry_type);
    let display_size = format_display_size(tree.size, args, tree.entry_type);
    let file_date_size_details = match (display_datetime.is_empty(), display_size.is_empty()) {
        (true, true) => String::new(),
        (true, false) | (false, true) => concat_str!("(", display_datetime, display_size, ") "),
        (false, false) => concat_str!("(", display_datetime, ", ", display_size, ") "),
    };

    if depth == 0 {
        let root_name = ansi_color!(&args.colors.root, bold=!args.is_grayscale, display_name);
        writeln!(writer, "{}", concat_str!(MARGIN_LEFT, root_name))?;
    } else {
        let (color, time_color, is_bold) = match tree.entry_type {
            EntryType::Directory => {
                counts.dir_count += 1;
                (&args.colors.dir, &args.colors.detail, !args.is_grayscale)
            }
            EntryType::File => {
                counts.file_count += 1;
                (
                    if args.is_grayscale || tree.path.is_none() {
                        &None
                    } else if tree.path.as_ref().map_or(false, |path| is_executable(path)) {
                        &args.colors.exec
                    } else {
                        &args.colors.file
                    },
                    &args.colors.detail,
                    false,
                )
            }
        };

        let connector_color = if depth == 1 { &args.colors.root } else { &args.colors.dir };
        let indent_bar = HORIZONTAL.repeat(args.indent) + " ";
        let connector = if args.is_flat {
            String::new()
        } else if is_last {
            ansi_color!(connector_color, bold=false, concat_str!(LAST_CONNECTOR, indent_bar))
        } else {
            ansi_color!(connector_color, bold=false, concat_str!(MID_CONNECTOR, indent_bar))
        };

        let enum_prefix = if args.is_enumerate && depth != 0 {
            ansi_color!(args.colors.detail, bold=false, concat_str!("[", enumeration, "] "))
        } else {
            String::new()
        };

        let entry_details = if file_date_size_details.is_empty() {
            file_date_size_details
        } else {
            ansi_color!(time_color, bold=false, file_date_size_details)
        };

        if tree.entry_type == EntryType::File && !tree.windows.is_empty() {
            let label_width = tree
                .fmt_width
                .unwrap_or_else(|| visible_width(&build_plain_path_line_label(display_name, tree.windows[0].line_number)));

            let first_label = build_colored_path_line_label(
                display_name,
                tree.windows[0].line_number,
                color,
                is_bold,
                color,
            );
            let first_label_width =
                visible_width(&build_plain_path_line_label(display_name, tree.windows[0].line_number));
            let first_gap = build_snippet_gap(label_width, first_label_width, args.is_window && !tree.windows[0].text.is_empty());

            writeln!(
                writer,
                "{}",
                concat_str!(
                    MARGIN_LEFT,
                    prefix,
                    connector,
                    enum_prefix,
                    entry_details,
                    first_label,
                    first_gap,
                    &tree.windows[0].text
                )
            )?;

            if tree.windows.len() > 1 {
                let repeated_prefix = build_repeated_match_prefix(
                    prefix,
                    is_last,
                    args,
                    &enum_prefix,
                    &entry_details,
                    connector_color,
                );

                for window in tree.windows.iter().skip(1) {
                    let repeated_label = build_colored_path_line_label(
                        display_name,
                        window.line_number,
                        &args.colors.muted,
                        false,
                        &args.colors.muted,
                    );
                    let repeated_label_width =
                        visible_width(&build_plain_path_line_label(display_name, window.line_number));
                    let repeated_gap =
                        build_snippet_gap(label_width, repeated_label_width, args.is_window && !window.text.is_empty());

                    writeln!(
                        writer,
                        "{}",
                        concat_str!(
                            MARGIN_LEFT,
                            &repeated_prefix,
                            repeated_label,
                            repeated_gap,
                            &window.text
                        )
                    )?;
                }
            }
        } else {
            let entry_name = ansi_color!(color, bold=is_bold, display_name);
            writeln!(
                writer,
                "{}",
                concat_str!(MARGIN_LEFT, prefix, connector, enum_prefix, entry_details, entry_name)
            )?;
        }
    }

    let level_indent = NB_SINGLE.repeat(args.indent) + " ";
    let new_prefix = if args.is_flat {
        String::new()
    } else if depth == 0 {
        prefix.to_string()
    } else if is_last {
        concat_str!(prefix, level_indent, " ")
    } else {
        let pipe_color = if depth == 1 { &args.colors.root } else { &args.colors.dir };
        concat_str!(prefix, ansi_color!(pipe_color, bold=false, SOLID_VERTICAL), level_indent)
    };

    tree.children.sort_by(|_, a, _, b| (args.sort_by)(a, b));

    let total_files = tree
        .children
        .values()
        .filter(|child| child.entry_type == EntryType::File)
        .count();

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

        if files_seen >= args.max_files {
            let trunc_num = total_files - args.max_files;
            if trunc_num > 0 {
                counts.file_count += trunc_num.saturating_sub(1);
                let trunc_fmt = concat_str!(trunc_num.to_string(), " more ...");
                let trunc_label = ansi_color!(&args.colors.detail, bold=false, trunc_fmt);
                tree.children.insert(
                    trunc_label.clone(),
                    Tree::new(
                        trunc_label.clone(),
                        trunc_label,
                        None,
                        EntryType::File,
                        None,
                        None,
                        None,
                        Vec::new(),
                    ),
                );
            }
        }
    }

    let last_index = tree.children.len().saturating_sub(1);
    for (index, child) in tree.children.values_mut().enumerate() {
        let is_last_child = index == last_index;
        let enumeration = if args.is_enumerate {
            let enum_padding = count_digits_log(last_index.saturating_add(1))
                .saturating_sub(count_digits_log(index.saturating_add(1)));
            concat_str!(" ".repeat(enum_padding), index.saturating_add(1).to_string())
        } else {
            String::new()
        };

        write_tree_to_buf(
            child,
            &enumeration,
            depth + 1,
            &new_prefix,
            is_last_child,
            args,
            counts,
            writer,
        )?;
    }

    if depth == 1 && is_last {
        writeln!(writer)?;
    }

    Ok(())
}

fn build_plain_path_line_label(display_name: &str, line_number: Option<usize>) -> String {
    match line_number {
        Some(line_number) => concat_str!(display_name, ":", line_number.to_string()),
        None => display_name.to_string(),
    }
}

fn build_colored_path_line_label(
    display_name: &str,
    line_number: Option<usize>,
    path_color: &Option<&'static str>,
    is_bold: bool,
    line_color: &Option<&'static str>,
) -> String {
    let colored_name = ansi_color!(path_color, bold=is_bold, display_name);
    match line_number {
        Some(line_number) => {
            let line_part = ansi_color!(line_color, bold=is_bold, concat_str!(":", line_number.to_string()));
            concat_str!(colored_name, line_part)
        }
        None => colored_name,
    }
}

fn build_snippet_gap(label_width: usize, label_actual_width: usize, has_text: bool) -> String {
    if !has_text {
        return String::new();
    }

    let padding = " ".repeat(label_width.saturating_sub(label_actual_width));
    concat_str!(padding, SEARCH_SNIPPET_SEPARATOR)
}

fn build_repeated_match_prefix(
    prefix: &str,
    is_last: bool,
    args: &RippyArgs,
    enum_prefix: &str,
    entry_details: &str,
    connector_color: &Option<&'static str>,
) -> String {
    if args.is_flat {
        return concat_str!(prefix, " ".repeat(visible_width(enum_prefix) + visible_width(entry_details)));
    }

    let connector_width = args.indent + 2;
    let repeated_connector = if is_last {
        " ".repeat(connector_width)
    } else {
        concat_str!(
            ansi_color!(connector_color, bold=false, SOLID_VERTICAL),
            " ".repeat(connector_width.saturating_sub(1))
        )
    };

    concat_str!(
        prefix,
        repeated_connector,
        " ".repeat(visible_width(enum_prefix) + visible_width(entry_details))
    )
}

pub fn print_tree(tree: &mut Tree, args: &RippyArgs, counts: &mut TreeCounts) -> io::Result<()> {
    let stdout = stdout();
    let mut writer = io::BufWriter::new(stdout.lock());
    write_tree_to_buf(tree, "", 0, "", true, args, counts, &mut writer)
}

pub fn count_tree(tree: &Tree, counts: &mut TreeCounts, is_first: bool) {
    match tree.entry_type {
        EntryType::Directory => {
            if !is_first {
                counts.dir_count += 1;
            }
        }
        EntryType::File => counts.file_count += 1,
    }

    for child in tree.children.values() {
        count_tree(child, counts, false);
    }
}

fn strip_ansi(input: &str) -> String {
    ANSI_REGEX.replace_all(input, "").to_string()
}

fn visible_width(input: &str) -> usize {
    strip_ansi(input).chars().count()
}