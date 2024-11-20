// #![allow(unused)]
#![allow(non_upper_case_globals)]
mod args;
mod tcolor;
mod tree;
mod crawl;

use std::sync::LazyLock;

fn main() -> std::io::Result<()> {
    // Initialize global args
    static args: LazyLock<crate::args::RippyArgs> = LazyLock::new(|| crate::args::parse_args());

    // Starts timer if show elapsed present
    let start = if args.show_elapsed { Some(std::time::Instant::now()) } else { None };

    match crate::crawl::crawl_directory(&args) {
        Ok(result) => {
            let num_matched = result.paths.len();
            let num_searched = result.paths_searched;
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
                match tree.write_to_json_file(&args) {
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
            let mut fmt_result = crate::args::format_result_summary(&args, num_matched, num_searched, &counts);
    
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
