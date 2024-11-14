use std::io::IsTerminal;
use std::path::PathBuf;

use crate::tcolor::{RippySchema, enable_ansi_support, ERROR_COLOR, WARN_COLOR};
use crate::{ansi_color, concat_str};
use crate::tree::{Tree, TreeCounts};

use clap::{value_parser, Arg, ArgAction, Command};
use regex::{Regex, RegexSet};

/// Returns the full version and build info for rippy in the format of:
/// 
/// ```text
/// v<VERSION> (<BUILD_DATE>)
/// ```
/// 
/// To be used after help menu to display equivalent of program about or program version.
const RELEASE_INFO: Option<&str> = option_env!("RELEASE_INFO");


/// Sorting keys and whether or not they're in ascending (true) or descending (false) order.
#[derive(Debug, PartialEq, Eq)]
pub enum SortKey {
    Date(bool),
    Name(bool),
    Size(bool),
    Type(bool),
}

impl SortKey {
     /// Provides sorting comparator by through `sort_key.compare()(a: &Tree, b: &Tree)` as a static function to sort children field of `Tree`.
     pub fn compare(&self) -> fn(&Tree, &Tree) -> std::cmp::Ordering {
         match self {
               SortKey::Date(true) => |a: &Tree, b: &Tree| a.last_modified.unwrap_or_default().partial_cmp(&b.last_modified.unwrap_or_default()).unwrap_or(std::cmp::Ordering::Equal),
               SortKey::Date(false) => |a: &Tree, b: &Tree| a.last_modified.unwrap_or_default().partial_cmp(&b.last_modified.unwrap_or_default()).unwrap_or(std::cmp::Ordering::Equal).reverse(),
               SortKey::Size(true) => |a: &Tree, b: &Tree| a.size.cmp(&b.size),
               SortKey::Size(false) => |a: &Tree, b: &Tree| a.size.cmp(&b.size).reverse(),
               SortKey::Type(true) => |a: &Tree, b: &Tree| a.entry_type.cmp(&b.entry_type),
               SortKey::Type(false) => |a: &Tree, b: &Tree| a.entry_type.cmp(&b.entry_type).reverse(),
               SortKey::Name(true) => |a: &Tree, b: &Tree| a.name.cmp(&b.name),
               SortKey::Name(false) => |a: &Tree, b: &Tree| a.name.cmp(&b.name).reverse(),
         }
     }
 }

/// Primary struct holding all rippy arguments after parsing to expected types
#[derive(Debug)]
pub struct RippyArgs {
    pub directory: PathBuf,
    pub pattern: Option<Regex>,
    pub is_search: bool,
    pub ignore_patterns: Option<RegexSet>,
    pub include_all: bool,
    pub include_patterns: Option<RegexSet>,
    pub max_depth: usize,
    pub max_files: usize,
    pub output: String,
    pub indent: usize,
    pub sort_by: fn(&Tree, &Tree) -> std::cmp::Ordering,
    pub is_dir_detail: bool,
    pub show_full_path: bool,
    pub show_relative_path: bool,
    pub show_size: bool,
    pub show_date: bool,
    pub is_short_date: bool,
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
/// Parses command line arguments and returns as struct to use as config container throughout rippy.
pub fn parse_args() -> RippyArgs {
    let matches = Command::new("rippy")
        .version(RELEASE_INFO.unwrap_or("Unknown"))
        .author("Ante Tonkovic-Capin")
        .about(concat_str!(env!("CARGO_PKG_NAME"), " ", option_env!("RELEASE_INFO").unwrap_or("[unknown version]"), "\nCrawls directory specified according to arguments, optionally executing multithreaded searches for pattern provided, returning results in a pruned and pretty printed terminal tree."))
        .disable_version_flag(true)
        .disable_help_flag(true)
        .after_help("For example, run `rippy \"./\"` to display a tree of the current directory's contents.")
        /* Positional arguments */
        .arg(Arg::new("directory")
             .help("Sets the root directory to search")
             .value_name("DIRECTORY")
             .required(true)
             .index(1))
        .arg(Arg::new("pattern")
             .help("Sets the pattern to search file contents for")
             .value_name("PATTERN")
             .index(2))
          /* Optional arguments */
        .arg(Arg::new("all")
             .short('A')
             .short_alias('a')
             .long("all")
             .action(ArgAction::SetTrue)
             .display_order(0)
             .help("Include hidden files and directories"))      
        .arg(Arg::new("sort-by")
             .short('B')
             .short_alias('b')
             .long("sort-by")
             .alias("sort")
             .value_name("KEY")
             .default_value("name")
             .hide_default_value(true)
             .hide_possible_values(true)
             .value_parser(["date","name","size","type"])
             .ignore_case(true)
             .display_order(1)
             .action(ArgAction::Set)
             .help("Sorting options: \"date\", \"name\" [d], \"size\" or \"type\""))
        .arg(Arg::new("max-depth")
             .short('L')
             .long("max-depth")
             .value_name("DEPTH")
             .action(ArgAction::Set)
             .display_order(2)
             .value_parser(value_parser!(usize))
             .help("Maximum directory depth to search"))    
        .arg(Arg::new("ignore")
             .short('I')
             .short_alias('i')
             .long("ignore")
             .value_name("PAT1, ..., PATN")
             .value_delimiter(',')
             .display_order(3)
             .action(ArgAction::Append)
             .help("Ignore specific file extensions or directories"))         
        .arg(Arg::new("include")
             .short('X')
             .short_alias('x')
             .long("include")
             .value_name("PAT1, ..., PATN")
             .value_delimiter(',')
             .display_order(4)
             .action(ArgAction::Append)
             .help("Restrict search to specific filename patterns"))                  
        .arg(Arg::new("window-radius")
             .short('R')
             .short_alias('r')
             .long("window-radius")
             .value_name("RADIUS")
             .default_value("20")
             .value_parser(value_parser!(usize))
             .hide_default_value(true)
             .display_order(5)
             .action(ArgAction::Set)
             .help("Maximum character radius for result snippet window"))                        
        .arg(Arg::new("max-files")
             .short('M')
             .short_alias('m')
             .long("max-files")
             .value_name("FILES")
             .action(ArgAction::Set)
             .display_order(6)
             .value_parser(value_parser!(usize))
             .help("Maximum number of files to display for each directory"))          
        .arg(Arg::new("output")
             .short('O')
             .short_alias('o')
             .long("output")
             .value_name("FILENAME")
             .action(ArgAction::Set)
             .display_order(7)
             .help("Export the results as JSON to specified file"))       
        .arg(Arg::new("indent")
             .short('N')
             .short_alias('n')
             .long("indent")
             .value_name("WIDTH")
             .action(ArgAction::Set)
             .value_parser(value_parser!(usize))
             .default_value("2")
             .hide_default_value(true)
             .display_order(8)
             .help("Character width to use for tree depth indentation"))         
        .arg(Arg::new("case-insensitive")
             .short('C')
             .short_alias('c')
             .long("case-insensitive")
             .action(ArgAction::SetTrue)
             .display_order(9)
             .help("Make pattern matching case insensitive"))     
        .arg(Arg::new("follow-links")
             .short('l')
             .long("follow-links")
             .action(ArgAction::SetTrue)
             .display_order(10)
             .help("Follow targets of symbolic links when found"))                                           
        .arg(Arg::new("relative-path")
             .short('P')
             .short_alias('p')
             .long("relative-path")
             .action(ArgAction::SetTrue)
             .help("Display the relative paths from root with results"))
        .arg(Arg::new("reverse")
             .short('Z')
             .short_alias('z')
             .long("reverse")
             .action(ArgAction::SetTrue)
             .help("Reverses sort order from ascending to descending"))             
        .arg(Arg::new("full-path")
             .short('K')
             .short_alias('k')
             .long("full-path")
             .action(ArgAction::SetTrue)
             .help("Display the full canonical paths with results"))             
        .arg(Arg::new("size")
             .short('S')
             .short_alias('s')
             .long("size")
             .action(ArgAction::SetTrue)
             .help("Display the size of files and directories with results"))
        .arg(Arg::new("date")
             .short('D')
             .short_alias('d')
             .long("date")
             .action(ArgAction::SetTrue)
             .help("Display the system last modified datetime with results"))      
         .arg(Arg::new("short-date")
             .short('Y')
             .short_alias('y')
             .long("short-date")
             .action(ArgAction::SetTrue)
             .help("Display a shortened last modified date as YYYY-MM-DD"))                         
        .arg(Arg::new("enumerate")
             .short('E')
             .short_alias('e')
             .long("enumerate")
             .action(ArgAction::SetTrue)
             .help("Display results enumerated by index within parent")) 
         .arg(Arg::new("time")
             .short('T')
             .short_alias('t')
             .long("time")
             .action(ArgAction::SetTrue)
             .help("Display the search duration time with results"))     
        .arg(Arg::new("no-gitignore")
             .short('g')
             .long("no-gitignore")
             .aliases(["gitignore","no-ignore"])
             .action(ArgAction::SetTrue)
             .help("Do not use .gitignore files when found for filtering"))         
        .arg(Arg::new("gray")
             .short('G')
             .long("gray")
             .alias("grayscale")
             .action(ArgAction::SetTrue)
             .help("Display the results in grayscale without styling")) 
        .arg(Arg::new("quote")
             .short('Q')
             .short_alias('q')
             .long("quote")
             .action(ArgAction::SetTrue)
             .help("Display the path results wrapped in double-quotes"))   
        .arg(Arg::new("flat")
             .short('F')
             .short_alias('f')
             .long("flat")
             .action(ArgAction::SetTrue)
             .help("Display the results as flat list without indentation"))                   
        .arg(Arg::new("dir-detail")
             .short('U')
             .short_alias('u')
             .long("dir-detail")
             .action(ArgAction::SetTrue)
             .help("Display size and date time details for directories"))     
        .arg(Arg::new("windowless")
             .short('W')
             .short_alias('w')
             .long("windowless")
             .action(ArgAction::SetTrue)
             .help("Display search results without context snippet window"))   
        .arg(Arg::new("just-counts")
            .short('J')
            .short_alias('j')
            .long("just-counts")
            .alias("counts")
            .action(ArgAction::SetTrue)
            .help("Display just entry counts without rendering a tree"))     
        .arg(Arg::new("version")
            .short('v')
            .short_alias('V')
            .long("version")
            .action(ArgAction::SetTrue)
            .help("Display the version of rippy")
            .display_order(1000)
            .action(clap::ArgAction::Version))
        .arg(Arg::new("help")
            .short('h')
            .long("help")
            .action(ArgAction::SetTrue)
            .help("Display help and usage information for rippy")
            .display_order(1000)
            .action(clap::ArgAction::Help))        
        .get_matches();

    // Initial start directory to crawl
    let directory_arg = matches.get_one::<String>("directory").map_or_else(|| ".".to_string(), |p| p.replace("\\", "/"));
    let directory = PathBuf::from(&directory_arg);

    // Exit if only required argument, <directory>, does not exist or is not a valid directory to traverse
    if !directory.exists() || !directory.is_dir() {
        let error_fmt = ansi_color!(ERROR_COLOR, bold=true, "error:"); // (241, 76, 76)
        let directory_fmt = ansi_color!(WARN_COLOR, bold=false, directory_arg); // (229, 229, 16)
        eprintln!("{} The directory provided, '{}', does not exist or is not a valid directory.", error_fmt, directory_fmt);
        std::process::exit(1);
    }
     // Show full path, not used with verbose by default
     let show_full_path = matches.get_flag("full-path");
     // Show full relative paths
     let show_relative_path = matches.get_flag("relative-path");

     // Allows avoiding calling on dir entries since dir entry paths are derived from root path using 'rootpath + filename' approach
     let directory = if show_full_path {
          PathBuf::from(std::path::absolute(std::path::Path::new(&directory)).map_or(directory_arg.to_owned(), |path| path.to_string_lossy().replace("\\","/")))
     } else {
          directory
     };

    // Pattern to search for in file contents
    let is_ignore_case = matches.get_flag("case-insensitive");
    let pattern = matches.get_one::<String>("pattern").map_or_else(|| None, |pat| {if is_ignore_case {Some(Regex::new(&concat_str!("(?i)", &pat)).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e)).unwrap())} else {Some(Regex::new(&pat).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e)).unwrap())}});
    let is_search = pattern.is_some();
    let ignore_patterns: Option<RegexSet> = matches.get_many::<String>("ignore").map_or_else(|| None, |v| Some(parse_and_convert_patterns(v.collect::<Vec<_>>(), is_ignore_case)));
    let include_patterns: Option<RegexSet> = matches.get_many::<String>("include").map_or_else(|| None, |v| Some(parse_and_convert_patterns(v.collect::<Vec<_>>(), is_ignore_case)));
    
    // Include hidden and other directories set to be ignored by default
    let include_all = matches.get_flag("all");

    // Max directory depth to search
    let max_depth = *matches.get_one::<usize>("max-depth").unwrap_or(&usize::MAX);
    
    // Max files to display within each directory
    let max_files = *matches.get_one::<usize>("max-files").unwrap_or(&usize::MAX);

    // Output tree as JSON to specified file
    let output = matches.get_one::<String>("output").map_or_else(|| "".to_string(), |s| s.to_string());

    // Indentation width to use for new level when displaying tree
    let indent = *matches.get_one::<usize>("indent").unwrap_or(&2_usize);

    // Use reversed sort ordering
    let reverse = matches.get_flag("reverse");

    // Sort tree by ordering
    let sort_by =  match matches.get_one::<String>("sort-by").unwrap_or(&"name".to_string()).to_lowercase().as_ref() {
          "date" => SortKey::Date(!reverse).compare(),
          "name" => SortKey::Name(!reverse).compare(),
          "size" => SortKey::Size(!reverse).compare(),
          "type" => SortKey::Type(!reverse).compare(),
               _ => SortKey::Name(!reverse).compare(),
     };

    // Display dir-detail details for both file and directory types
    let is_dir_detail = matches.get_flag("dir-detail");

    // Override defaults and use all available details
    let is_verbose = matches.get_flag("verbose");

    // Determine if size should be displayed
    let show_size = matches.get_flag("size") || is_verbose;

    // Show last modified date only in short format
    let is_short_date = matches.get_flag("short-date");
    let show_date = matches.get_flag("date") || is_short_date || is_verbose;

    // Elapsed search time
    let show_elapsed = matches.get_flag("time") || is_verbose;

    // Select color schema based on arguments and ansi support and if search pattern is present
    let is_grayscale = matches.get_flag("gray") || !std::io::stdout().is_terminal() || !enable_ansi_support();
    let colors: RippySchema = RippySchema::get_color_schema(is_grayscale);

    // Use double-quotes when displaying paths
    let is_quote = matches.get_flag("quote");
    
    // Display tree as flattened list
    let is_flat = matches.get_flag("flat");

    // Development addition to display just summary counts without rendering tree
    let is_just_counts = matches.get_flag("just-counts");

    // Follow symbolic links when found if target points to directory
    let is_follow_links = matches.get_flag("follow-links");

    // Display enumerated position of entry within parent directory
    let is_enumerate = matches.get_flag("enumerate");

    // Whether or not gitignore files should be used to filter results using specified globs and patterns
    let is_gitignore = !matches.get_flag("no-gitignore"); // More like asking "is no gitignore flag present? If not, then yes is gitignore, false otherwise"

    // Display context window with search results and character radius window if present, assuming a window was requested if radius is specified without explicit window flag
    let is_window = !matches.get_flag("windowless");
    let radius = *matches.get_one::<usize>("window-radius").unwrap_or(&20_usize);

    RippyArgs {
        directory,
        pattern,
        is_search,
        ignore_patterns,
        include_all,
        include_patterns,
        max_depth,
        max_files,
        output,
        indent,
        sort_by,
        is_dir_detail,
        show_full_path,
        show_relative_path,
        show_size,
        show_date,
        is_short_date,
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
        colors
    }
}

/// Parses and converts the Vec<String> of arguments collected from "ignore" or "pattern" into regex sets based on wildcards present
fn parse_and_convert_patterns(patterns: Vec<&String>, case_insensitive: bool) -> RegexSet {
     let converted_patterns: Vec<String> = patterns.into_iter().filter(|s| !s.is_empty()).map(|s| {
         let pattern = if s.contains('*') {
              concat_str!("^", regex::escape(&s).replace(r"\*", ".*"), "$")
           } else {
              concat_str!("^", regex::escape(&s), "$")
         };
         if case_insensitive {
             concat_str!("(?i)", pattern)
         } else {
             pattern
         }
     }).collect();
     let re_set = RegexSet::new(converted_patterns).expect("Invalid regex patterns");
     re_set
}

/// Summarizes and formats result returned by args after `tree` has been constructed and rendered
pub fn format_result_summary(args: &'static RippyArgs, num_matched: usize, num_searched: usize, counts: &TreeCounts) -> String {
     let fmt_result = if num_matched > 0 {
          if args.is_search {
              let match_suffix = if num_matched != 1 {"matches"} else {"match"};
              let match_text = concat_str!(num_matched.to_string(), " ", match_suffix);
              let match_fmt = ansi_color!(&args.colors.window, bold=!args.is_grayscale, &match_text);
              let search_text = concat_str!(num_searched.to_string(), " searched");
              let search_fmt = ansi_color!(&args.colors.search, bold=false, &search_text);
              concat_str!(match_fmt, ", ", search_fmt)
          } else {
              let dirs_suffix = if counts.dir_count != 1 {"directories"} else {"directory"};
              let dirs_text = concat_str!(counts.dir_count.to_string(), " ", dirs_suffix);
              let dirs_fmt = ansi_color!(&args.colors.dir, bold=!args.is_grayscale, &dirs_text);
              let files_suffix = if counts.file_count != 1 {"files"} else {"file"};
              let files_text = concat_str!(counts.file_count.to_string(), " ", files_suffix);
              let files_fmt = ansi_color!(&args.colors.file, bold=!args.is_grayscale, &files_text);
              concat_str!(dirs_fmt, ", ", files_fmt)
          }
      } else {
          if args.is_search {
              let matches_fmt = ansi_color!(&args.colors.zero, bold=!args.is_grayscale, "0 matches");
              let searched_fmt = ansi_color!(&args.colors.search, bold=false, concat_str!(num_searched.to_string(), " searched"));
              concat_str!({if args.is_just_counts {""} else {"\n"}}, matches_fmt, ", ", searched_fmt)
          } else {
              let dirs_text = concat_str!(counts.dir_count.to_string(), " directories");
              let dirs_fmt = ansi_color!(&args.colors.dir, bold=!args.is_grayscale, &dirs_text);
              let files_text = concat_str!(counts.file_count.to_string(), " files");
              let files_fmt = ansi_color!(&args.colors.file, bold=!args.is_grayscale, &files_text);
              concat_str!({if args.is_just_counts {""} else {"\n"}}, &dirs_fmt, ", ", &files_fmt)
          }
      };
      // Return result after summary counts formatted
      fmt_result
}
