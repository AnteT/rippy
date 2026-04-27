use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::PathBuf;

use clap::parser::ValueSource;
use clap::{value_parser, Arg, ArgAction, ArgGroup, ArgMatches, Command};
use regex::{Regex, RegexSet};

use crate::error::RippyError;
use crate::tcolor::{enable_ansi_support, RippySchema};
use crate::tree::{Tree, TreeCounts};
use crate::{ansi_color, concat_str};

const RELEASE_INFO: Option<&str> = option_env!("RELEASE_INFO");

#[derive(Debug, PartialEq, Eq)]
pub enum SortKey {
    Date(bool),
    Name(bool),
    Size(bool),
    Type(bool),
}

impl SortKey {
    pub fn compare(&self) -> fn(&Tree, &Tree) -> std::cmp::Ordering {
        match self {
            SortKey::Date(true) => |a: &Tree, b: &Tree| {
                a.last_modified
                    .unwrap_or_default()
                    .partial_cmp(&b.last_modified.unwrap_or_default())
                    .unwrap_or(std::cmp::Ordering::Equal)
            },
            SortKey::Date(false) => |a: &Tree, b: &Tree| {
                a.last_modified
                    .unwrap_or_default()
                    .partial_cmp(&b.last_modified.unwrap_or_default())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .reverse()
            },
            SortKey::Size(true) => |a: &Tree, b: &Tree| a.size.cmp(&b.size),
            SortKey::Size(false) => |a: &Tree, b: &Tree| a.size.cmp(&b.size).reverse(),
            SortKey::Type(true) => |a: &Tree, b: &Tree| a.entry_type.cmp(&b.entry_type),
            SortKey::Type(false) => |a: &Tree, b: &Tree| a.entry_type.cmp(&b.entry_type).reverse(),
            SortKey::Name(true) => |a: &Tree, b: &Tree| a.name.cmp(&b.name),
            SortKey::Name(false) => |a: &Tree, b: &Tree| a.name.cmp(&b.name).reverse(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchTerm {
    pub source: String,
    pub regex: Regex,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub terms: Vec<SearchTerm>,
    pub groups: Vec<Vec<usize>>,
}

impl SearchQuery {
    pub fn is_match(&self, matched_terms: &[bool]) -> bool {
        self.groups
            .iter()
            .any(|group| group.iter().all(|term_id| matched_terms.get(*term_id).copied().unwrap_or(false)))
    }

    pub fn matching_term_ids(&self, matched_terms: &[bool]) -> Vec<usize> {
        let mut matched_ids = Vec::new();

        for group in &self.groups {
            let group_matches = group
                .iter()
                .all(|term_id| matched_terms.get(*term_id).copied().unwrap_or(false));

            if group_matches {
                for term_id in group {
                    if matched_terms.get(*term_id).copied().unwrap_or(false) && !matched_ids.contains(term_id) {
                        matched_ids.push(*term_id);
                    }
                }
            }
        }

        matched_ids
    }

    pub fn term_count(&self) -> usize {
        self.terms.len()
    }
}

#[derive(Debug)]
pub struct RippyArgs {
    pub directory: PathBuf,
    pub search: Option<SearchQuery>,
    pub is_search: bool,
    pub ignore_patterns: Option<RegexSet>,
    pub include_all: bool,
    pub include_patterns: Option<RegexSet>,
    pub max_depth: usize,
    pub max_files: usize,
    pub max_display_matches: usize,
    pub output: String,
    pub indent: usize,
    pub sort_by: fn(&Tree, &Tree) -> std::cmp::Ordering,
    pub is_dir_detail: bool,
    pub is_dirs_only: bool,
    pub show_full_path: bool,
    pub show_relative_path: bool,
    pub show_size: bool,
    pub show_date: bool,
    pub show_line_numbers: bool,
    pub date_format: String,
    pub show_elapsed: bool,
    pub is_grayscale: bool,
    pub is_quote: bool,
    pub is_flat: bool,
    pub is_window: bool,
    pub is_just_counts: bool,
    pub is_enumerate: bool,
    pub is_follow_links: bool,
    pub is_gitignore: bool,
    pub radius: usize,
    pub colors: RippySchema,
}

pub fn parse_args(args: Option<Vec<String>>) -> Result<RippyArgs, RippyError> {
    let raw_args = args.unwrap_or_else(|| std::env::args().collect::<Vec<_>>());
    let matches = build_command()
        .try_get_matches_from(raw_args)
        .map_err(RippyError::Cli)?;

    let directory_arg = matches
        .get_one::<String>("directory")
        .map_or_else(|| ".".to_string(), |p| p.replace('\\', "/"));
    let mut directory = PathBuf::from(&directory_arg);

    if !directory.exists() || !directory.is_dir() {
        return Err(RippyError::InvalidDirectory { path: directory_arg });
    }

    let show_full_path = matches.get_flag("full-path");
    let show_relative_path = matches.get_flag("relative-path");

    if show_full_path {
        directory = PathBuf::from(
            std::path::absolute(directory.as_path())
                .map_or_else(|_| directory.to_string_lossy().to_string(), |path| {
                    path.to_string_lossy().replace('\\', "/")
                }),
        );
    }

    let is_ignore_case = matches.get_flag("case-insensitive");
    let search = build_search_query(&matches, is_ignore_case)?;
    let is_search = search.is_some();

    let ignore_patterns = matches
        .get_many::<String>("ignore")
        .map(|values| values.collect::<Vec<_>>())
        .map(|values| parse_and_convert_patterns(values, is_ignore_case, "--ignore"))
        .transpose()?;

    let include_patterns = matches
        .get_many::<String>("include")
        .map(|values| values.collect::<Vec<_>>())
        .map(|values| parse_and_convert_patterns(values, is_ignore_case, "--include"))
        .transpose()?;

    let include_all = matches.get_flag("all");
    let max_depth = *matches.get_one::<usize>("max-depth").unwrap_or(&usize::MAX);
    let max_files = *matches.get_one::<usize>("max-files").unwrap_or(&usize::MAX);
    let output = matches
        .get_one::<String>("output")
        .map_or_else(String::new, |value| value.to_string());
    let indent = *matches.get_one::<usize>("indent").unwrap_or(&2_usize);
    let reverse = matches.get_flag("reverse");

    let sort_by = match matches
        .get_one::<String>("sort-by")
        .map(String::as_str)
        .unwrap_or("name")
        .to_ascii_lowercase()
        .as_str()
    {
        "date" => SortKey::Date(!reverse).compare(),
        "name" => SortKey::Name(!reverse).compare(),
        "size" => SortKey::Size(!reverse).compare(),
        "type" => SortKey::Type(!reverse).compare(),
        _ => SortKey::Name(!reverse).compare(),
    };

    let is_dir_detail = matches.get_flag("dir-detail");
    let is_dirs_only = matches.get_flag("dirs-only");
    let show_size = matches.get_flag("size");
    let date_format = matches
        .get_one::<String>("date-format")
        .map_or_else(|| "%Y-%m-%d %H:%M:%S".to_string(), |fmt| fmt.to_string());
    let show_date = matches.get_flag("date")
        || matches.value_source("date-format") == Some(ValueSource::CommandLine);
    let show_elapsed = matches.get_flag("time");
    let is_grayscale = matches.get_flag("gray")
        || !std::io::stdout().is_terminal()
        || !enable_ansi_support();
    let colors = RippySchema::get_color_schema(is_grayscale);
    let is_quote = matches.get_flag("quote");
    let is_flat = matches.get_flag("flat");
    let is_just_counts = matches.get_flag("just-counts");
    let is_follow_links = matches.get_flag("follow-links");
    let is_enumerate = matches.get_flag("enumerate");
    let is_gitignore = !matches.get_flag("no-gitignore");
    let is_window = !matches.get_flag("windowless");
    let radius = *matches.get_one::<usize>("window-radius").unwrap_or(&20_usize);
    let show_line_numbers = !matches.get_flag("no-line-numbers");

    let max_display_matches = resolve_max_display_matches(&matches)?;

    Ok(RippyArgs {
        directory,
        search,
        is_search,
        ignore_patterns,
        include_all,
        include_patterns,
        max_depth,
        max_files,
        max_display_matches,
        output,
        indent,
        sort_by,
        is_dir_detail,
        is_dirs_only,
        show_full_path,
        show_relative_path,
        show_size,
        show_date,
        show_line_numbers,
        date_format,
        show_elapsed,
        is_grayscale,
        is_quote,
        is_flat,
        is_window,
        is_just_counts,
        is_enumerate,
        is_follow_links,
        is_gitignore,
        radius,
        colors,
    })
}

fn build_command() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(RELEASE_INFO.unwrap_or("Unknown"))
        .author("Ante Tonkovic-Capin")
        .about(concat_str!(
            env!("CARGO_PKG_NAME"),
            " ",
            option_env!("RELEASE_INFO").unwrap_or("[unknown version]"),
            "\nCrawls directory specified according to arguments, optionally executing multithreaded searches for patterns provided, returning results in a pruned and pretty printed terminal tree."
        ))
        .disable_version_flag(true)
        .disable_help_flag(true)
        .group(ArgGroup::new("search-root").args(["pattern", "search"]).multiple(false))
        .after_help("For example, run `rippy . -L 3` to display a tree of the current directory's contents to a max depth of 3 levels.")
        .arg(
            Arg::new("directory")
               .help("Sets the root directory to search")
               .value_name("DIRECTORY")
               .required(false)
               .default_value(".")
               .index(1),
        )
        .arg(
            Arg::new("pattern")
                .help("Sets the base pattern to search file contents for (backward-compatible positional form)")
                .value_name("PATTERN")
                .index(2),
        )
        .arg(
            Arg::new("search")
                .long("search")
                .short('f')
                .alias("find")
                .value_name("PATTERN")
                .action(ArgAction::Set)
                .help("Sets the base pattern to search file contents for"),
        )
        .arg(
            Arg::new("and")
                .long("and")
                .value_name("PATTERN")
                .action(ArgAction::Append)
                .help("Require an additional pattern to match in the same file"),
        )
        .arg(
            Arg::new("or")
                .long("or")
                .value_name("PATTERN")
                .action(ArgAction::Append)
                .help("Start a new alternative pattern group"),
        )
        .arg(
            Arg::new("all")
                .short('A')
                .short_alias('a')
                .long("all")
                .action(ArgAction::SetTrue)
                .display_order(0)
                .help("Include hidden files and directories"),
        )
        .arg(
            Arg::new("sort-by")
                .short('B')
                .short_alias('b')
                .long("sort-by")
                .alias("sort")
                .value_name("KEY")
                .default_value("name")
                .hide_default_value(true)
                .hide_possible_values(true)
                .value_parser(["date", "name", "size", "type"])
                .ignore_case(true)
                .display_order(1)
                .action(ArgAction::Set)
                .help("Sorting options: 'date', 'name' [d], 'size' or 'type'"),
        )
        .arg(
            Arg::new("max-depth")
                .short('L')
                .long("max-depth")
                .value_name("DEPTH")
                .action(ArgAction::Set)
                .display_order(2)
                .value_parser(value_parser!(usize))
                .help("Maximum directory depth to search"),
        )
        .arg(
            Arg::new("date-format")
                .short('Y')
                .short_alias('y')
                .long("date-format")
                .aliases(["date-formatting", "dt-format", "strftime", "format", "last-modified-format"])
                .value_name("FORMAT")
                .default_value("%Y-%m-%d %H:%M:%S")
                .hide_default_value(true)
                .action(ArgAction::Set)
                .display_order(3)
                .help("Display date using the specified format (e.g., '%Y-%m-%d')"),
        )
        .arg(
            Arg::new("ignore")
                .short('I')
                .short_alias('i')
                .long("ignore")
                .value_name("PAT1, ..., PATN")
                .value_delimiter(',')
                .display_order(4)
                .action(ArgAction::Append)
                .help("Ignore specific file extensions or directories"),
        )
        .arg(
            Arg::new("include")
                .short('X')
                .short_alias('x')
                .long("include")
                .value_name("PAT1, ..., PATN")
                .value_delimiter(',')
                .display_order(5)
                .action(ArgAction::Append)
                .help("Restrict search to specific filename patterns"),
        )
        .arg(
            Arg::new("max-display-matches")
                .short('m')
                .long("max-display-matches")
                .aliases(["max-results", "max-matches", "max-search-results", "max-search", "max-searches"])
                .value_name("COUNT")
                .action(ArgAction::Set)
                .display_order(6)
                .value_parser(value_parser!(usize))
                .help("Maximum number of search match windows to display per file"),
        )        
        .arg(
            Arg::new("window-radius")
                .short('R')
                .short_alias('r')
                .long("window-radius")
                .aliases(["radius", "window-size"])
                .value_name("RADIUS")
                .default_value("20")
                .value_parser(value_parser!(usize))
                .hide_default_value(true)
                .display_order(7)
                .action(ArgAction::Set)
                .help("Maximum character radius for result snippet window"),
        )        
        .arg(
            Arg::new("max-files")
                .short('M')
                .long("max-files")
                .value_name("FILES")
                .aliases(["max-contents", "max-file"])
                .action(ArgAction::Set)
                .display_order(8)
                .value_parser(value_parser!(usize))
                .help("Maximum number of files to display for each directory"),
        )
        .arg(
            Arg::new("all-matches")
                .long("all-matches")
                .action(ArgAction::SetTrue)
                .help("Display every matching window for a file unless capped by --max-display-matches"),
        )
        .arg(
            Arg::new("dirs-only")
                .long("dirs-only")
                .aliases(["dirs","dir-only","directories-only"])
                .action(ArgAction::SetTrue)
                .help("Display only directories in the tree"),
        )
        .arg(
            Arg::new("output")
                .short('O')
                .short_alias('o')
                .long("output")
                .value_name("FILENAME")
                .action(ArgAction::Set)
                .display_order(8)
                .help("Export the results as JSON to specified file"),
        )
        .arg(
            Arg::new("indent")
                .short('N')
                .short_alias('n')
                .long("indent")
                .value_name("WIDTH")
                .action(ArgAction::Set)
                .value_parser(value_parser!(usize))
                .default_value("2")
                .hide_default_value(true)
                .display_order(9)
                .help("Character width to use for tree depth indentation"),
        )
        .arg(
            Arg::new("case-insensitive")
                .short('C')
                .short_alias('c')
                .long("case-insensitive")
                .aliases(["uncase", "uncased", "ignore-case"])
                .action(ArgAction::SetTrue)
                .display_order(10)
                .help("Make pattern matching case insensitive"),
        )
        .arg(
            Arg::new("follow-links")
                .short('l')
                .long("follow-links")
                .aliases(["follow-symbolic-links", "follow"])
                .action(ArgAction::SetTrue)
                .display_order(11)
                .help("Follow targets of symbolic links when found"),
        )
        .arg(
            Arg::new("relative-path")
                .short('P')
                .short_alias('p')
                .long("relative-path")
                .aliases(["relative", "rel"])
                .action(ArgAction::SetTrue)
                .help("Display the relative paths from root with results"),
        )
        .arg(
            Arg::new("reverse")
                .short('Z')
                .short_alias('z')
                .long("reverse")
                .aliases(["reversed", "rev"])
                .action(ArgAction::SetTrue)
                .help("Reverses sort order from ascending to descending"),
        )
        .arg(
            Arg::new("full-path")
                .short('K')
                .short_alias('k')
                .long("full-path")
                .aliases(["absolute", "absolute-path"])
                .action(ArgAction::SetTrue)
                .help("Display the full canonical paths with results"),
        )
        .arg(
            Arg::new("size")
                .short('S')
                .short_alias('s')
                .long("size")
                .aliases(["show-size", "display-size"])
                .action(ArgAction::SetTrue)
                .help("Display the size of files and directories with results"),
        )
        .arg(
            Arg::new("date")
                .short('D')
                .short_alias('d')
                .long("date")
                .aliases(["last-modified", "datetime", "show-date", "display-date"])
                .action(ArgAction::SetTrue)
                .help("Display the system last modified datetime with results"),
        )
        .arg(
            Arg::new("enumerate")
                .short('E')
                .short_alias('e')
                .long("enumerate")
                .aliases(["enum", "enumerate", "indexed"])
                .action(ArgAction::SetTrue)
                .help("Display results enumerated by index within parent"),
        )
        .arg(
            Arg::new("time")
                .short('T')
                .short_alias('t')
                .long("time")
                .aliases(["show-elapsed", "search-time", "elapsed"])
                .action(ArgAction::SetTrue)
                .help("Display the search duration time with results"),
        )
        .arg(
            Arg::new("no-gitignore")
                .short('g')
                .long("no-gitignore")
                .aliases(["gitignore", "no-ignore"])
                .action(ArgAction::SetTrue)
                .help("Do not use .gitignore files when found for filtering"),
        )
        .arg(
            Arg::new("gray")
                .short('G')
                .long("gray")
                .aliases(["grayscale", "bw", "black-and-white", "no-color", "colorless"])
                .action(ArgAction::SetTrue)
                .help("Display the results in grayscale without styling"),
        )
        .arg(
            Arg::new("quote")
                .short('Q')
                .short_alias('q')
                .long("quote")
                .action(ArgAction::SetTrue)
                .help("Display the path results wrapped in double-quotes"),
        )
        .arg(
            Arg::new("flat")
                .short('F')
                .long("flat")
                .aliases(["flattened", "flatten"])
                .action(ArgAction::SetTrue)
                .help("Display the results as flat list without indentation"),
        )
        .arg(
            Arg::new("dir-detail")
                .short('U')
                .short_alias('u')
                .long("dir-detail")
                .aliases(["include-dir", "directory-detail"])
                .action(ArgAction::SetTrue)
                .help("Display size and date time details for directories"),
        )
        .arg(
            Arg::new("windowless")
                .short('W')
                .short_alias('w')
                .long("windowless")
                .aliases(["no-window", "without-window"])
                .action(ArgAction::SetTrue)
                .help("Display search results without context snippet window"),
        )
        .arg(
            Arg::new("no-line-numbers")
                .long("no-line-numbers")
                .aliases(["without-line-numbers", "hide-line-numbers", "no-lines", "no-line", "hide-line", "hide-lines"])
                .action(ArgAction::SetTrue)
                .help("Hide line numbers from search result windows"),
        )
        .arg(
            Arg::new("just-counts")
                .short('J')
                .short_alias('j')
                .long("just-counts")
                .aliases(["counts", "count", "counts-only"])
                .action(ArgAction::SetTrue)
                .help("Display just entry counts without rendering a tree"),
        )
        .arg(
            Arg::new("version")
                .short('v')
                .short_alias('V')
                .long("version")
                .help("Display the version of rippy")
                .display_order(1000)
                .action(ArgAction::Version),
        )
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::Help)
                .help("Display help and usage information for rippy")
                .display_order(1000),
        )
}

fn resolve_max_display_matches(matches: &ArgMatches) -> Result<usize, RippyError> {
    if let Some(value) = matches.get_one::<usize>("max-display-matches") {
        if *value == 0 {
            return Err(RippyError::InvalidValue {
                flag: "--max-display-matches",
                value: value.to_string(),
                reason: "expected a value greater than 0".to_string(),
            });
        }
        return Ok(*value);
    }

    if matches.get_flag("all-matches") {
        Ok(usize::MAX)
    } else {
        Ok(1)
    }
}

enum SearchToken {
    Base { raw: String, context: &'static str },
    And { raw: String, context: &'static str },
    Or { raw: String, context: &'static str },
}

fn build_search_query(matches: &ArgMatches, case_insensitive: bool) -> Result<Option<SearchQuery>, RippyError> {
    let mut tokens: Vec<(usize, SearchToken)> = Vec::new();

    if let Some(index) = matches.index_of("pattern") {
        if let Some(raw) = matches.get_one::<String>("pattern") {
            tokens.push((
                index,
                SearchToken::Base {
                    raw: raw.to_string(),
                    context: "search pattern",
                },
            ));
        }
    }

    if let Some(index) = matches.index_of("search") {
        if let Some(raw) = matches.get_one::<String>("search") {
            tokens.push((
                index,
                SearchToken::Base {
                    raw: raw.to_string(),
                    context: "--search",
                },
            ));
        }
    }

    if let (Some(indices), Some(values)) = (matches.indices_of("and"), matches.get_many::<String>("and")) {
        for (index, raw) in indices.zip(values) {
            tokens.push((
                index,
                SearchToken::And {
                    raw: raw.to_string(),
                    context: "--and",
                },
            ));
        }
    }

    if let (Some(indices), Some(values)) = (matches.indices_of("or"), matches.get_many::<String>("or")) {
        for (index, raw) in indices.zip(values) {
            tokens.push((
                index,
                SearchToken::Or {
                    raw: raw.to_string(),
                    context: "--or",
                },
            ));
        }
    }

    if tokens.is_empty() {
        return Ok(None);
    }

    tokens.sort_by_key(|(index, _)| *index);

    if !matches!(tokens.first(), Some((_, SearchToken::Base { .. }))) {
        return Err(RippyError::SearchExpression(
            "Search operators require a base search pattern from [PATTERN] or --search.".to_string(),
        ));
    }

    let mut terms: Vec<SearchTerm> = Vec::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current_group: Vec<usize> = Vec::new();
    let mut term_cache: HashMap<String, usize> = HashMap::new();
    let mut saw_base = false;

    for (_, token) in tokens {
        match token {
            SearchToken::Base { raw, context } => {
                if saw_base {
                    return Err(RippyError::SearchExpression(
                        "Only one base search pattern may be provided. Use --and and --or to extend it."
                            .to_string(),
                    ));
                }
                let term_id = get_or_compile_search_term(&mut terms, &mut term_cache, raw, context, case_insensitive)?;
                current_group.push(term_id);
                saw_base = true;
            }
            SearchToken::And { raw, context } => {
                if !saw_base {
                    return Err(RippyError::SearchExpression(
                        "`--and` requires a base search pattern from [PATTERN] or --search.".to_string(),
                    ));
                }
                let term_id = get_or_compile_search_term(&mut terms, &mut term_cache, raw, context, case_insensitive)?;
                current_group.push(term_id);
            }
            SearchToken::Or { raw, context } => {
                if !saw_base {
                    return Err(RippyError::SearchExpression(
                        "`--or` requires a base search pattern from [PATTERN] or --search.".to_string(),
                    ));
                }
                groups.push(std::mem::take(&mut current_group));
                let term_id = get_or_compile_search_term(&mut terms, &mut term_cache, raw, context, case_insensitive)?;
                current_group.push(term_id);
            }
        }
    }

    if current_group.is_empty() {
        return Err(RippyError::SearchExpression(
            "The search expression did not contain a usable pattern.".to_string(),
        ));
    }

    groups.push(current_group);

    Ok(Some(SearchQuery { terms, groups }))
}

fn get_or_compile_search_term(
    terms: &mut Vec<SearchTerm>,
    cache: &mut HashMap<String, usize>,
    raw: String,
    context: &'static str,
    case_insensitive: bool,
) -> Result<usize, RippyError> {
    if let Some(existing) = cache.get(&raw) {
        return Ok(*existing);
    }

    let regex_source = if case_insensitive {
        concat_str!("(?i)", &raw)
    } else {
        raw.clone()
    };

    let regex = Regex::new(&regex_source).map_err(|source| RippyError::InvalidRegex {
        context,
        pattern: raw.clone(),
        source,
    })?;

    let id = terms.len();
    terms.push(SearchTerm {
        source: raw.clone(),
        regex,
    });
    cache.insert(raw, id);
    Ok(id)
}

fn parse_and_convert_patterns(
    patterns: Vec<&String>,
    case_insensitive: bool,
    context: &'static str,
) -> Result<RegexSet, RippyError> {
    let converted_patterns: Vec<String> = patterns
        .into_iter()
        .filter(|pattern| !pattern.is_empty())
        .map(|pattern| {
            let wildcard_pattern = if pattern.contains('*') {
                concat_str!("^", regex::escape(pattern).replace(r"\*", ".*"), "$")
            } else {
                concat_str!("^", regex::escape(pattern), "$")
            };

            if case_insensitive {
                concat_str!("(?i)", &wildcard_pattern)
            } else {
                wildcard_pattern
            }
        })
        .collect();

    RegexSet::new(&converted_patterns).map_err(|source| RippyError::InvalidPatternList {
        context,
        pattern: converted_patterns.join(", "),
        source,
    })
}

pub fn format_result_summary(args: &RippyArgs, num_matched: usize, num_searched: usize, counts: &TreeCounts) -> String {
    if num_matched > 0 {
        if args.is_search {
            let match_suffix = if num_matched == 1 { "match" } else { "matches" };
            let match_text = concat_str!(num_matched.to_string(), " ", match_suffix);
            let match_fmt = ansi_color!(&args.colors.window, bold=!args.is_grayscale, &match_text);
            let search_text = concat_str!(num_searched.to_string(), " searched");
            let search_fmt = ansi_color!(&args.colors.search, bold=false, &search_text);
            concat_str!(match_fmt, ", ", search_fmt)
        } else {
            let dirs_suffix = if counts.dir_count == 1 { "directory" } else { "directories" };
            let dirs_text = concat_str!(counts.dir_count.to_string(), " ", dirs_suffix);
            let dirs_fmt = ansi_color!(&args.colors.dir, bold=!args.is_grayscale, &dirs_text);
            let files_suffix = if counts.file_count == 1 { "file" } else { "files" };
            let files_text = concat_str!(counts.file_count.to_string(), " ", files_suffix);
            let files_fmt = ansi_color!(&args.colors.file, bold=!args.is_grayscale, &files_text);
            concat_str!(dirs_fmt, ", ", files_fmt)
        }
    } else if args.is_search {
        let matches_fmt = ansi_color!(&args.colors.zero, bold=!args.is_grayscale, "0 matches");
        let searched_fmt = ansi_color!(
            &args.colors.search,
            bold=false,
            concat_str!(num_searched.to_string(), " searched")
        );
        concat_str!({ if args.is_just_counts { "" } else { "\n" } }, matches_fmt, ", ", searched_fmt)
    } else {
        let dirs_text = concat_str!(counts.dir_count.to_string(), " directories");
        let dirs_fmt = ansi_color!(&args.colors.dir, bold=!args.is_grayscale, &dirs_text);
        let files_text = concat_str!(counts.file_count.to_string(), " files");
        let files_fmt = ansi_color!(&args.colors.file, bold=!args.is_grayscale, &files_text);
        concat_str!({ if args.is_just_counts { "" } else { "\n" } }, dirs_fmt, ", ", files_fmt)
    }
}