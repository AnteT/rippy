use is_executable::IsExecutable;
use jwalk::WalkDirGeneric;
use crate::args::RippyArgs;
use crate::{ansi_color, concat_str};
use crate::ignorer::Ignorer;

// const DEFAULT_IGNORE: [&str;3] = ["venv", "node_modules", "__pycache__"];

#[derive(Debug, Clone, Default)] // Derive Serialize and Deserialize
pub struct TreeLeaf {
    pub name: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub last_modified: Option<f64>,
    pub size: Option<u64>,
    pub window: Option<String>,
    pub display: String, // New display field to preformat the needed string earlier
    pub is_sym: bool, // New for coloring sym links correctly when displayed
}
impl TreeLeaf {
    /// Create new `TreeLeaf`
    pub fn new(name: impl Into<String>, relative_path: impl Into<String>, is_dir: bool, last_modified: Option<f64>, size: Option<u64>, window: Option<String>, display: impl Into<String>, is_sym: bool ) -> TreeLeaf {
        TreeLeaf { name: name.into(), relative_path: relative_path.into(), is_dir, last_modified, size, window, display: display.into(), is_sym }
    }
}
// Implement Display for EntryType to convert to string
impl std::fmt::Display for TreeLeaf {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Summarized by Type, Name, Relative Path, Size, Full Path
        write!(f, "TreeLeaf({}, \"{}\")", if self.is_dir {"Directory"} else {"File"}, self.relative_path)
    }
}

#[derive(Debug, Clone)]
/// Primary container for directory crawl results.
pub struct CrawlResults {
    pub paths: Vec<TreeLeaf>,
    pub paths_searched: usize,
}

/// Primary directory crawl, returns `CrawlResults` struct containing Vec<TreeLeaf>.
pub fn crawl_directory(args: &'static RippyArgs) -> std::io::Result<CrawlResults> {
    let walk_dir = WalkDirGeneric::<(Ignorer, TreeLeaf)>::new(&args.directory)
        .skip_hidden(false) // Modified from `skip_hidden(!args.include_all)` after new ignorer.rs module and process added.
        .max_depth(args.max_depth)
        .follow_links(args.is_follow_links)
        .process_read_dir(|_depth, _path, ignorer, children| {
            
            // 1. Custom filter first pass
            children.retain(|dir_entry_result| {
                dir_entry_result.as_ref().map_or(false, |dir_entry| {
                    // Convert the file name to a string slice
                    dir_entry.file_name().to_str()
                        .map_or(false, |fname| {
                            let dir_entry_path = dir_entry.path();
                            let dir_entry_ftype = dir_entry.file_type;
                            let is_ftype_dir = dir_entry_ftype.is_dir() || ( dir_entry_ftype.is_symlink() && dir_entry_path.is_dir() );
                            let is_ftype_file = dir_entry_ftype.is_file() || ( dir_entry_ftype.is_symlink() && dir_entry_path.is_dir() );
                            let is_hidden_file = _depth.is_some() && fname.starts_with(".");

                            if is_hidden_file && args.is_gitignore && fname == ".gitignore" {
                                // Grab the .gitignore file now unless user wants to include all
                                *ignorer = Ignorer::new(&dir_entry_path);
                            }
                            // Separated checks for hidden file and gitignored file
                            if !args.include_all && is_hidden_file {
                                return false
                            }
                            // Needs to be ignored irrespective of file or directory type
                            if ignorer.is_ignore(&dir_entry_path, is_ftype_dir) 
                                || args.ignore_patterns.as_ref().map_or(false, |patterns| patterns.is_match(fname)) {
                                // println!("Skipped due to mathcing ignore glob: {:?}", dir_entry_path);
                                return false
                            }
                            // Return true for dirs that have already passed ignore check
                            if is_ftype_dir {
                                return true
                            } else {
                                // Result of boolean checks for passing include if is file or return false by boolean fail if filetype is not resolved
                                return is_ftype_file && args.include_patterns.as_ref().map_or(true, |patterns| patterns.is_match(fname)) 
                            }
                        }) // Defaults to false if file_name is None or to_str fails
                }) // Defaults to false if dir_entry_result is Err
            });

            // 2. Custom filter second pass if needed due to gitignore initialization point
            if args.is_gitignore && ignorer.has_matcher() {
                children.retain(|dir_entry_result| {
                    dir_entry_result.as_ref().map_or(false, |dir_entry| {
                        let dir_entry_ftype = dir_entry.file_type;
                        let is_ftype_dir = dir_entry_ftype.is_dir() || ( dir_entry_ftype.is_symlink() && dir_entry.path().is_dir() );
                        // Results in skipping those entries that may have been missed in first retention check due to timing of gitignore instantiation
                        !ignorer.is_ignore(&dir_entry.path(), is_ftype_dir)
                    })
                });
            }

            // 3. Create the client state for entries we intend to keep and build the tree from
            children.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    // Let symlinks fall through since its cheaper to let the File::open fail than to check through a syscall and traverse to find out if its a file or not
                    let window_snippet: Option<String> = if !args.is_search || dir_entry.file_type().is_dir() { None } else {
                        let re = args.pattern.as_ref().unwrap(); // if args.is_search then args.pattern will have valid Regex else Error would've been raised during args parsing.
                        let snippet_from_file_read: Option<String> = if let Ok(contents) = std::fs::read_to_string(dir_entry.path()) {
                            if re.is_match(&contents) {
                                if args.is_window {
                                    if let Some(mat) = re.find(&contents) {
                                        // Snippet extraction begins here
                                        let line_start = contents[..mat.start()].rfind(&['\r', '\n']).map(|pos| pos + 1).unwrap_or(0);
                                        let line_end = contents[mat.end()..].find(&['\r', '\n']).map(|pos| mat.end() + pos).unwrap_or(contents.len());
                                        let snippet_start = if mat.start() > line_start + args.radius { mat.start() - args.radius } else { line_start };
                                        let snippet_end = if mat.end() + args.radius < line_end { mat.end() + args.radius } else { line_end };
                                        let snippet_start_adjusted = if snippet_start < line_start { line_start } else { snippet_start };
                                        let snippet_end_adjusted = if snippet_end > line_end { line_end } else { snippet_end };
                                        // Ensure we slice at valid UTF-8 boundaries
                                        let valid_snippet_start = if contents.is_char_boundary(snippet_start_adjusted) {
                                            snippet_start_adjusted
                                        } else {
                                            contents.char_indices().take_while(|&(i, _)| i < snippet_start_adjusted).last().map(|(i, _)| i).unwrap_or(snippet_start_adjusted)
                                        };
                                        let valid_snippet_end = if contents.is_char_boundary(snippet_end_adjusted) {
                                            snippet_end_adjusted
                                        } else {
                                            contents.char_indices().take_while(|&(i, _)| i < snippet_end_adjusted).last().map(|(i, c)| i + c.len_utf8()).unwrap_or(snippet_end_adjusted)
                                        };
                                        let valid_snippet = &contents[valid_snippet_start..valid_snippet_end];
                                        let match_start_index = mat.start() - valid_snippet_start;
                                        let match_end_index = mat.end() - valid_snippet_start;
                                        let snippet_mark = 
                                            ansi_color!(&args.colors.muted, bold=false, &valid_snippet[..match_start_index].trim_start().to_owned()) +
                                            &ansi_color!(&args.colors.window, bold=!args.is_grayscale, &valid_snippet[match_start_index..match_end_index]) +
                                            &ansi_color!(&args.colors.muted, bold=false, valid_snippet[match_end_index..].trim_end());
                                        let end_elipses = if snippet_end != line_end {ansi_color!(&args.colors.muted, bold=false, "...")} else {"".to_string()};
                                        let start_elipses = if snippet_start != line_start {ansi_color!(&args.colors.muted, bold=false, "...")} else {"".to_string()};
                                        let snippet_fmt = start_elipses.to_owned() + &snippet_mark + &end_elipses;
                                            // Snippet extraction ends, return matched snippet
                                            Some(snippet_fmt)
                                        } else {
                                            // File still matched but unable to find snippet due to reading contents to string
                                            Some("".to_string())
                                        }
                                } else {
                                    // File matches search pattern but no snippet needed due to args
                                    Some("".to_string())
                                }
                            } else {
                                // No match due to `re.is_match()` is False
                                None
                            }
                        } else {
                            // File read error from `if let Ok(contents) = std::fs::read_to_string(path)`
                            None 
                        };
                    // Gets assigned to `window_snippet` on line ~86
                    snippet_from_file_read
                    };

                    if !args.is_search || dir_entry.file_type().is_dir() || window_snippet.is_some() || ( dir_entry.file_type().is_symlink() && dir_entry.path().is_dir() ) {
                        let is_symbolic = dir_entry.file_type().is_symlink();
                        let name = dir_entry.file_name().to_string_lossy().to_string();
                        let relative_path = dir_entry.path().to_string_lossy().replace("\\", "/");
                        let entry_path = dir_entry.path();
                        
                        let last_modified = if args.show_date {
                            dir_entry.metadata().map_or(Some(0_f64), |m| m.modified().ok().and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok().map(|duration| duration.as_secs_f64())))
                        } else {
                            None
                        };
                        let size = if args.show_size {
                            dir_entry.metadata().map_or(Some(0_u64), |m| Some(m.len()))
                        } else {
                            None
                        };
                        let is_dir = dir_entry.file_type().is_dir() || ( is_symbolic && entry_path.is_dir() );
                        let display = if args.show_relative_path || args.show_full_path { &relative_path } else { &name };
                        let display = if args.is_quote { &concat_str!("\"", display, "\"") } else { display };
                        let display = if is_symbolic {
                            let sym_path = std::fs::read_link(&entry_path)
                            .map_or("[unable to resolve]".to_string(), |p| { 
                                let (color, is_bold) = if is_dir {
                                    (args.colors.dir, !args.is_grayscale)
                                } else if p.is_executable() || entry_path.is_executable() {
                                    (args.colors.exec, false)
                                } else {
                                    (args.colors.file, false)
                                };
                                let sym_display = if args.show_relative_path || args.show_full_path { p.to_string_lossy().replace("\\", "/") } else {p.file_name().map_or_else(|| p.to_string_lossy().replace("\\", "/"), |p| p.to_string_lossy().replace("\\", "/"))};
                                let sym_display = if args.is_quote {concat_str!("\"", sym_display, "\"")} else {sym_display};
                                // Now we have it as a string with the right color scheme and display style
                                let sym_display = ansi_color!(color, bold=is_bold, sym_display);
                                sym_display
                                }
                            );
                            &concat_str!(ansi_color!(args.colors.sym, bold=is_dir && !args.is_grayscale, display), " -> ", sym_path)
                        } else {
                            display
                        };
                        dir_entry.client_state = TreeLeaf::new(&name, &relative_path, is_dir, last_modified, size, window_snippet, display, is_symbolic);
                    }
                }
            });
        });

    let mut paths: Vec<TreeLeaf> = Vec::new();
    let mut paths_searched:usize = 0;

    for entry_result in walk_dir {
        let entry = entry_result.unwrap();
        if entry.file_type().is_file() && entry.depth > 0 {
            paths_searched += 1;
        }
        // Skip entry if its the root dir or if we're searching for matching patterns and none was found or if we're targeting specific file patterns and the empty dir has no matches and itself doesnt match the pattern
        if entry.depth() == 0 || (args.is_search && entry.client_state.window.is_none()) || (entry.client_state.is_dir && args.include_patterns.as_ref().map_or(false, |patterns| !patterns.is_match(&entry.file_name().to_string_lossy().to_string()))) {
            // DEBUG only:
            // println!("Entry skipped at depth [{}]: {:?} with client state: {:?}", entry.depth, entry.file_name(), entry.client_state);
            continue;
        } else {          
            paths.push(entry.client_state);
        }
    }
    Ok( CrawlResults { paths, paths_searched } )
}