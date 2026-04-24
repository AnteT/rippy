#![allow(non_upper_case_globals)]

use std::process::ExitCode;

use rippy::args;
use rippy::crawl;
use rippy::error::RippyError;
use rippy::tree;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(RippyError::Cli(err)) => {
            let code = if err.use_stderr() { 2 } else { 0 };
            let _ = err.print();
            ExitCode::from(code)
        }
        Err(error) => {
            eprintln!("{}", error.format_pretty());
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), RippyError> {
    let parsed_args = args::parse_args(None)?;
    let args: &'static args::RippyArgs = Box::leak(Box::new(parsed_args));

    let start = if args.show_elapsed {
        Some(std::time::Instant::now())
    } else {
        None
    };

    let result = crawl::crawl_directory(args)?;
    let num_matched = result.paths.len();
    let num_searched = result.paths_searched;
    let mut tree = tree::build_tree_from_paths(result.paths, args);

    if args.show_size && args.is_dir_detail {
        tree.calculate_sizes();
    }

    if args.is_search && args.is_window && !args.is_dirs_only {
        tree.calculate_fmt_width(args);
    }

    if !args.output.is_empty() {
        tree.write_to_json_file(args)
            .map_err(|source| RippyError::io("Failed writing output to file", Some(args.output.clone()), source))?;
    }

    let mut counts = tree::TreeCounts::new();

    if args.is_just_counts {
        tree::count_tree(&tree, &mut counts, true);
    } else {
        tree::print_tree(&mut tree, args, &mut counts)
            .map_err(|source| RippyError::io("Failed writing tree output", None, source))?;
    }

    let mut fmt_result = args::format_result_summary(args, num_matched, num_searched, &counts);
    if let Some(time) = start {
        fmt_result = format!("{} ({:.3}s)", fmt_result, time.elapsed().as_secs_f32());
    }

    println!("{fmt_result}");
    Ok(())
}