// #![allow(unused)]
#![allow(non_upper_case_globals)]
mod args;
mod tcolor;
mod tree;
mod dir;

use std::sync::LazyLock;

fn main() -> std::io::Result<()> {

    // Initialize global args
    static args: LazyLock<crate::args::RippyArgs> = LazyLock::new(|| crate::args::parse_args());

    // Starts timer if show elapsed present
    let start = if args.show_elapsed { Some(std::time::Instant::now()) } else { None };

    match crate::dir::crawl_directory(&args) {
        Ok(result) => {
            let num_matched = result.paths.len();
            let num_searched = result.paths_searched.to_string();
            let mut tree = crate::tree::build_tree_from_paths(result.paths, &args);

            // Only calculate dir sizes if needed based on is_dir_detail argument present
            if args.show_size && args.is_dir_detail {
                tree.calculate_sizes();
            }

            // Calculate format width for window snippets if arg present
            if args.is_search && args.is_window {
                tree.calculate_fmt_width();
            }

            // Output tree as JSON to file provided
            if !args.output.is_empty() {
                match tree.write_to_json_file(&args.output, &args) {
                    Ok(_) => {},
                    Err(e) => eprintln!("{} writing output to file: {}", ansi_color!(crate::tcolor::ERROR_COLOR, bold=true, "Error"), e),
                }
            } 
                        
            // Tracking entry counts
            let mut counts = crate::tree::TreeCounts::new();
            
            // Print primary tree with results if not just counts present
            if args.is_just_counts {
                crate::tree::count_tree(&tree, &mut counts, true);
            } else {
                crate::tree::print_tree(&mut tree, &args, &mut counts)?;
            }
            // Big things have small beginnings...
            let mut fmt_result: String;

            if num_matched > 0 {
                if args.is_search {
                    let match_suffix = if num_matched != 1 {"matches"} else {"match"};
                    let match_text = concat_str!(num_matched.to_string(), " ", match_suffix);
                    let match_fmt = ansi_color!(&args.colors.window, bold=!args.is_grayscale, &match_text);
                    let search_text = concat_str!(num_searched, " searched");
                    let search_fmt = ansi_color!(&args.colors.search, bold=false, &search_text);
                    fmt_result = concat_str!(match_fmt, ", ", search_fmt);
                } else {
                    let dirs_suffix = if counts.dir_count != 1 {"directories"} else {"directory"};
                    let dirs_text = concat_str!(counts.dir_count.to_string(), " ", dirs_suffix);
                    let dirs_fmt = ansi_color!(&args.colors.dir, bold=!args.is_grayscale, &dirs_text);
                    let files_suffix = if counts.file_count != 1 {"files"} else {"file"};
                    let files_text = concat_str!(counts.file_count.to_string(), " ", files_suffix);
                    let files_fmt = ansi_color!(&args.colors.file, bold=!args.is_grayscale, &files_text);
                    fmt_result = concat_str!(dirs_fmt, ", ", files_fmt);
                }
            } else {
                if args.is_search {
                    let matches_fmt = ansi_color!(&args.colors.zero, bold=!args.is_grayscale, "0 matches");
                    let searched_fmt = ansi_color!(&args.colors.search, bold=false, concat_str!(num_searched, " searched"));
                    fmt_result = concat_str!({if args.is_just_counts {""} else {"\n"}}, matches_fmt, ", ", searched_fmt);
                } else {
                    let dirs_text = concat_str!(counts.dir_count.to_string(), " directories");
                    let dirs_fmt = ansi_color!(&args.colors.dir, bold=!args.is_grayscale, &dirs_text);
                    let files_text = concat_str!(counts.file_count.to_string(), " files");
                    let files_fmt = ansi_color!(&args.colors.file, bold=!args.is_grayscale, &files_text);
                    fmt_result = concat_str!({if args.is_just_counts {""} else {"\n"}}, &dirs_fmt, ", ", &files_fmt);
                }
            }
    
            fmt_result = match start {
                Some(time) => format!("{} ({:.3}s)", fmt_result, time.elapsed().as_secs_f32()),
                None => fmt_result
            };
    
            // Print the rendered tree
            println!("{fmt_result}");
    
        },
        Err(e) => {
            eprintln!("{} reading directory: {}", ansi_color!(crate::tcolor::ERROR_COLOR, bold=true, "Error"), e)
        }
    }
    Ok(())
}
