use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use is_executable::IsExecutable;
use jwalk::WalkDirGeneric;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::args::RippyArgs;
use crate::error::RippyError;
use crate::{ansi_color, concat_str};

#[derive(Clone, Debug, Default)]
pub struct Ignorer {
    pub matcher: Option<ignore::gitignore::Gitignore>,
}

impl Ignorer {
    pub fn new<P: AsRef<std::path::Path>>(gitignore_path: P) -> Self {
        Ignorer {
            matcher: Some(ignore::gitignore::Gitignore::new(gitignore_path).0),
        }
    }

    pub fn is_ignore<P: AsRef<std::path::Path>>(&self, path: P, is_dir: bool) -> bool {
        self.matcher
            .as_ref()
            .map_or(false, |matcher| matcher.matched(path, is_dir).is_ignore())
    }

    pub fn has_matcher(&self) -> bool {
        self.matcher.is_some()
    }
}

impl<P: AsRef<std::path::Path>> From<P> for Ignorer {
    fn from(value: P) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchWindow {
    pub line_number: Option<usize>,
    pub text: String,
}

impl MatchWindow {
    pub fn new(line_number: Option<usize>, text: impl Into<String>) -> Self {
        Self {
            line_number,
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TreeLeaf {
    pub name: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub last_modified: Option<f64>,
    pub size: Option<u64>,
    pub windows: Vec<MatchWindow>,
    pub display: String,
    pub is_sym: bool,
}

impl TreeLeaf {
    pub fn new(
        name: impl Into<String>,
        relative_path: impl Into<String>,
        is_dir: bool,
        last_modified: Option<f64>,
        size: Option<u64>,
        windows: Vec<MatchWindow>,
        display: impl Into<String>,
        is_sym: bool,
    ) -> TreeLeaf {
        TreeLeaf {
            name: name.into(),
            relative_path: relative_path.into(),
            is_dir,
            last_modified,
            size,
            windows,
            display: display.into(),
            is_sym,
        }
    }
}

impl std::fmt::Display for TreeLeaf {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "TreeLeaf({}, \"{}\")",
            if self.is_dir { "Directory" } else { "File" },
            self.relative_path
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrawlResults {
    pub paths: Vec<TreeLeaf>,
    pub paths_searched: usize,
}

pub fn crawl_directory(args: &'static RippyArgs) -> Result<CrawlResults, RippyError> {
    let walk_dir = WalkDirGeneric::<(Ignorer, TreeLeaf)>::new(&args.directory)
        .skip_hidden(false)
        .max_depth(args.max_depth)
        .follow_links(args.is_follow_links)
        .process_read_dir(|depth, _path, ignorer, children| {
            let mut requires_second_filter = false;

            children.retain(|dir_entry_result| {
                dir_entry_result.as_ref().map_or(false, |dir_entry| {
                    dir_entry.file_name().to_str().map_or(false, |fname| {
                        let dir_entry_path = dir_entry.path();
                        let dir_entry_ftype = dir_entry.file_type;
                        let is_ftype_dir =
                            dir_entry_ftype.is_dir() || (dir_entry_ftype.is_symlink() && dir_entry_path.is_dir());
                        let is_ftype_file =
                            dir_entry_ftype.is_file() || (dir_entry_ftype.is_symlink() && dir_entry_path.is_file());
                        let is_hidden_file = depth.is_some() && fname.starts_with('.');

                        if is_hidden_file && args.is_gitignore && fname == ".gitignore" {
                            *ignorer = Ignorer::new(&dir_entry_path);
                            requires_second_filter = true;
                        }

                        if !args.include_all && is_hidden_file {
                            return false;
                        }

                        if ignorer.is_ignore(&dir_entry_path, is_ftype_dir)
                            || args
                                .ignore_patterns
                                .as_ref()
                                .map_or(false, |patterns| patterns.is_match(fname))
                        {
                            return false;
                        }

                        if is_ftype_dir {
                            true
                        } else {
                            is_ftype_file
                                && args
                                    .include_patterns
                                    .as_ref()
                                    .map_or(true, |patterns| patterns.is_match(fname))
                        }
                    })
                })
            });

            if args.is_gitignore && ignorer.has_matcher() && requires_second_filter {
                children.retain(|dir_entry_result| {
                    dir_entry_result.as_ref().map_or(false, |dir_entry| {
                        let dir_entry_ftype = dir_entry.file_type;
                        let is_ftype_dir = dir_entry_ftype.is_dir()
                            || (dir_entry_ftype.is_symlink() && dir_entry.path().is_dir());
                        !ignorer.is_ignore(dir_entry.path(), is_ftype_dir)
                    })
                });
            }

            children.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    dir_entry.client_state = build_tree_leaf(dir_entry, args);
                }
            });
        });

    let mut paths: Vec<TreeLeaf> = Vec::new();
    let mut dir_entries: Vec<TreeLeaf> = Vec::new();
    let mut candidate_files: Vec<TreeLeaf> = Vec::new();
    let mut dir_map: HashMap<String, TreeLeaf> = HashMap::new();
    let mut paths_searched: usize = 0;

    for entry_result in walk_dir {
        let entry = entry_result.map_err(|error| RippyError::walk(error.to_string()))?;

        if entry.depth() == 0 {
            continue;
        }

        let state = entry.client_state;
        if state.is_dir {
            dir_map.insert(state.relative_path.clone(), state.clone());
            dir_entries.push(state.clone());

            if !args.is_search && args.is_dirs_only {
                paths.push(state);
            }
            continue;
        }

        paths_searched += 1;

        if args.is_search {
            candidate_files.push(state);
        } else if !args.is_dirs_only {
            paths.push(state);
        }
    }

    if !args.is_search {
        return Ok(CrawlResults { paths, paths_searched });
    }

    let mut matched_files: Vec<TreeLeaf> = candidate_files
        .into_par_iter()
        .filter_map(|mut leaf| match search_file(&leaf.relative_path, args) {
            Ok(Some(windows)) => {
                leaf.windows = windows;
                Some(leaf)
            }
            Ok(None) => None,
            Err(_) => None,
        })
        .collect();

    matched_files.sort_unstable_by(|a, b| a.relative_path.cmp(&b.relative_path));

    if !args.is_dirs_only {
        return Ok(CrawlResults {
            paths: matched_files,
            paths_searched,
        });
    }

    let mut matched_dir_paths: BTreeSet<String> = BTreeSet::new();
    for file in &matched_files {
        if let Some(parent) = Path::new(&file.relative_path).parent() {
            let parent_str = parent.to_string_lossy().replace('\\', "/");
            if !parent_str.is_empty() {
                matched_dir_paths.insert(parent_str);
            }
        }
    }

    let mut matched_dirs: Vec<TreeLeaf> = matched_dir_paths
        .into_iter()
        .filter_map(|path| dir_map.get(&path).cloned())
        .collect();

    matched_dirs.sort_unstable_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(CrawlResults {
        paths: matched_dirs,
        paths_searched,
    })
}

fn build_tree_leaf(dir_entry: &jwalk::DirEntry<(Ignorer, TreeLeaf)>, args: &RippyArgs) -> TreeLeaf {
    let entry_path = dir_entry.path();
    let is_symbolic = dir_entry.file_type().is_symlink();
    let is_dir = dir_entry.file_type().is_dir() || (is_symbolic && entry_path.is_dir());
    let name = dir_entry.file_name().to_string_lossy().to_string();
    let relative_path = entry_path.to_string_lossy().replace('\\', "/");

    let metadata = if args.show_date || args.show_size {
        dir_entry.metadata().ok()
    } else {
        None
    };

    let last_modified = if args.show_date {
        metadata
            .as_ref()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs_f64())
            .or(Some(0.0))
    } else {
        None
    };

    let size = if args.show_size {
        metadata.as_ref().map(|meta| meta.len()).or(Some(0))
    } else {
        None
    };

    let display = if args.show_relative_path || args.show_full_path {
        relative_path.clone()
    } else {
        name.clone()
    };
    let display = if args.is_quote {
        concat_str!("\"", display, "\"")
    } else {
        display
    };

    let display = if is_symbolic {
        let sym_path = std::fs::read_link(&entry_path).map_or_else(
            |_| "[unable to resolve]".to_string(),
            |path| {
                let (color, is_bold) = if is_dir {
                    (args.colors.dir, !args.is_grayscale)
                } else if path.is_executable() || entry_path.is_executable() {
                    (args.colors.exec, false)
                } else {
                    (args.colors.file, false)
                };

                let sym_display = if args.show_relative_path || args.show_full_path {
                    path.to_string_lossy().replace('\\', "/")
                } else {
                    path.file_name().map_or_else(
                        || path.to_string_lossy().replace('\\', "/"),
                        |file_name| file_name.to_string_lossy().replace('\\', "/"),
                    )
                };
                let sym_display = if args.is_quote {
                    concat_str!("\"", sym_display, "\"")
                } else {
                    sym_display
                };
                ansi_color!(color, bold=is_bold, sym_display)
            },
        );
        concat_str!(
            ansi_color!(args.colors.sym, bold=is_dir && !args.is_grayscale, display),
            " -> ",
            sym_path
        )
    } else {
        display
    };

    TreeLeaf::new(name, relative_path, is_dir, last_modified, size, Vec::new(), display, is_symbolic)
}

fn search_file(path: &str, args: &RippyArgs) -> std::io::Result<Option<Vec<MatchWindow>>> {
    let Some(query) = args.search.as_ref() else {
        return Ok(None);
    };

    let contents = std::fs::read_to_string(path)?;
    let mut matched_terms = vec![false; query.term_count()];
    let mut term_hits: Vec<Vec<(usize, usize)>> = vec![Vec::new(); query.term_count()];

    for (term_id, term) in query.terms.iter().enumerate() {
        for mat in term.regex.find_iter(&contents) {
            matched_terms[term_id] = true;
            term_hits[term_id].push((mat.start(), mat.end()));
        }
    }

    if !query.is_match(&matched_terms) {
        return Ok(None);
    }

    if !args.is_window && !args.show_line_numbers {
        return Ok(Some(Vec::new()));
    }

    let matching_term_ids = query.matching_term_ids(&matched_terms);

    let mut hits: Vec<(usize, usize)> = matching_term_ids
        .into_iter()
        .flat_map(|term_id| term_hits[term_id].iter().copied())
        .collect();

    hits.sort_unstable();
    hits.dedup();

    let display_limit = args.max_display_matches.min(hits.len());
    let displayed_hits: Vec<(usize, usize)> = hits.into_iter().take(display_limit).collect();

    let line_starts = build_line_starts(&contents);

    let windows = displayed_hits
        .into_iter()
        .map(|(start, end)| {
            let line_number = if args.show_line_numbers {
                Some(find_line_number(&line_starts, start))
            } else {
                None
            };
            let text = if args.is_window {
                format_match_window(&contents, start, end, args)
            } else {
                String::new()
            };
            MatchWindow::new(line_number, text)
        })
        .collect::<Vec<_>>();

    Ok(Some(windows))
}

fn build_line_starts(contents: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in contents.bytes().enumerate() {
        if byte == b'\n' && index + 1 < contents.len() {
            starts.push(index + 1);
        }
    }
    starts
}

fn find_line_number(line_starts: &[usize], byte_offset: usize) -> usize {
    line_starts.partition_point(|offset| *offset <= byte_offset)
}

fn format_match_window(
    contents: &str,
    match_start: usize,
    match_end: usize,
    args: &RippyArgs,
) -> String {
    let line_start = contents[..match_start]
        .rfind(&['\r', '\n'])
        .map(|position| position + 1)
        .unwrap_or(0);
    let line_end = contents[match_end..]
        .find(&['\r', '\n'])
        .map(|position| match_end + position)
        .unwrap_or(contents.len());

    let snippet_start = clamp_to_char_boundary_start(
        contents,
        std::cmp::max(line_start, match_start.saturating_sub(args.radius)),
    );
    let snippet_end = clamp_to_char_boundary_end(
        contents,
        std::cmp::min(line_end, match_end.saturating_add(args.radius)),
    );

    let snippet = &contents[snippet_start..snippet_end];
    let relative_match_start = match_start.saturating_sub(snippet_start).min(snippet.len());
    let relative_match_end = match_end.saturating_sub(snippet_start).min(snippet.len());

    let before = &snippet[..relative_match_start];
    let matched = &snippet[relative_match_start..relative_match_end];
    let after = &snippet[relative_match_end..];

    let before_fmt = ansi_color!(&args.colors.muted, bold=false, before.trim_start());
    let match_fmt = ansi_color!(&args.colors.window, bold=!args.is_grayscale, matched);
    let after_fmt = ansi_color!(&args.colors.muted, bold=false, after.trim_end());
    let start_ellipses = if snippet_start > line_start {
        ansi_color!(&args.colors.muted, bold=false, "...")
    } else {
        String::new()
    };
    let end_ellipses = if snippet_end < line_end {
        ansi_color!(&args.colors.muted, bold=false, "...")
    } else {
        String::new()
    };

    concat_str!(
        start_ellipses,
        before_fmt,
        match_fmt,
        after_fmt,
        end_ellipses
    )
}

fn clamp_to_char_boundary_start(input: &str, mut index: usize) -> usize {
    index = index.min(input.len());
    while index > 0 && !input.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn clamp_to_char_boundary_end(input: &str, mut index: usize) -> usize {
    index = index.min(input.len());
    while index < input.len() && !input.is_char_boundary(index) {
        index += 1;
    }
    index
}