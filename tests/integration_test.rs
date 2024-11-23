// Run tests with `cargo test --test integration_test` or to show output: `cargo test --test integration_test -- --show-output`
mod common;

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;
    use std::thread;
    use std::time::Duration;
    use std::path::PathBuf;
    use rippy::{crawl::{self, CrawlResults, TreeLeaf}, tcolor};
    use rippy::tree::{self, Tree, EntryType, TreeMap};
    use regex::{Regex, RegexSet};
    use serde_json::json;

    use crate::common::{generate_args_from, generate_tree_map, DirError, RootDirectory};

    #[test]
    /// Tests all major program arguments including search include & ignore patterns, primary search target, max depth and case sensitivity application.
    pub fn test_rippy_arguments() {
        let major_features = vec!["rippy", ".", "-i", "target", "-x", "*.rs,R*md", "-L", "20", "\\w[A-z]{3}find-me\\b", "-c"];
        let rip_args = generate_args_from(major_features);

        let expected_directory = std::path::PathBuf::from(".");
        assert_eq!(rip_args.directory, expected_directory);

        let expected_ignore_patterns = RegexSet::new(["(?i)^target$"]).unwrap();
        assert_eq!(rip_args.ignore_patterns.unwrap().patterns(), expected_ignore_patterns.patterns());

        let expected_include_patterns = RegexSet::new(["(?i)^.*\\.rs$", "(?i)^R.*md$"]).unwrap();
        assert_eq!(rip_args.include_patterns.unwrap().patterns(), expected_include_patterns.patterns());

        let expected_max_depth = 20_usize;
        assert_eq!(rip_args.max_depth, expected_max_depth);
        
        let expected_colors = tcolor::RippySchema::get_color_schema(false);
        assert_eq!(rip_args.colors, expected_colors);

        let expected_pattern = Regex::new("(?i)\\w[A-z]{3}find-me\\b").unwrap();

        assert_eq!(rip_args.pattern.unwrap().as_str(), expected_pattern.as_str());
        assert_eq!(rip_args.is_search, true);

        // Grayscale color schema test
        let test_grayscale = vec!["rippy", ".", "--grayscale"];
        let rip_args = generate_args_from(test_grayscale);        

        let expected_colors_grayscale = tcolor::RippySchema::get_color_schema(true);
        assert_eq!(rip_args.colors, expected_colors_grayscale);        
    }

    // #[test]
    // #[should_panic(expected = "not a valid directory")]
    /// TODO: Removed since no longer panics on invalid directory:
    /// Ensure that program panics when given an invalid directory as an argument.
    // pub fn test_panics_on_invalid_directory() {
        // generate_args_from(vec!["riplib", "../kdfjakfjdklf"]);
    // }

    #[test]
    /// Generates fake directory and produces crawl results equivalent to the tree:
    /// 
    /// ```shell
    /// fake-small
    /// ├── a
    /// │   ├── b
    /// │   │   ├── c
    /// │   │   │   ╰── file.txt
    /// │   │   ╰── file.txt
    /// │   ╰── file.txt
    /// ╰── file.txt
    ///
    ///3 directories, 4 files
    /// ```
    /// 
    /// Tests a deep and tall directory structure using default options.
    pub fn test_crawl_directory_tall() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-tall";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", ROOT_TEST_DIR]));
        let file_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("file.txt", file_contents)?;
        test_dir.generate("a/file.txt", file_contents)?;
        test_dir.generate("a/b/file.txt", file_contents)?;
        test_dir.generate("a/b/c/file.txt", file_contents)?;
        let expected_crawl_results = CrawlResults { 
            paths: vec![
                TreeLeaf {name: "a".to_string(),relative_path: "fake-tall/a".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "a".to_string(),is_sym: false,},
                TreeLeaf {name: "b".to_string(),relative_path: "fake-tall/a/b".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "b".to_string(),is_sym: false,},
                TreeLeaf {name: "c".to_string(),relative_path: "fake-tall/a/b/c".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "c".to_string(),is_sym: false,},
                TreeLeaf {name: "file.txt".to_string(),relative_path: "fake-tall/a/b/c/file.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "file.txt".to_string(),relative_path: "fake-tall/a/b/file.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "file.txt".to_string(),relative_path: "fake-tall/a/file.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "file.txt".to_string(),relative_path: "fake-tall/file.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.txt".to_string(),is_sym: false,},
            ],
            paths_searched: 4,
            };
        let crawl_results = crawl::crawl_directory(&ARGS);
        assert_eq!(crawl_results.unwrap(), expected_crawl_results);
        test_dir.clean()
    }

    #[test]
    /// Generates fake directory and produces crawl results equivalent to the tree:
    /// 
    /// ```shell
    /// fake-wide
    /// ├── a
    /// │   ╰── file.txt
    /// ├── b
    /// │   ╰── file.txt
    /// ├── c
    /// │   ╰── file.txt
    /// ╰── file.md
    ///
    ///3 directories, 4 files
    /// ```
    /// 
    /// Tests a wide and shallow directory structure using default options.
    pub fn test_crawl_directory_wide() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-wide";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", ROOT_TEST_DIR]));
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("file.md", no_contents)?;
        test_dir.generate("a/file.txt", no_contents)?;
        test_dir.generate("b/file.txt", no_contents)?;
        test_dir.generate("c/file.txt", no_contents)?;
        let expected_crawl_results = CrawlResults {
            paths: vec![
                TreeLeaf {name: "a".to_string(),relative_path: "fake-wide/a".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "a".to_string(),is_sym: false,},
                TreeLeaf {name: "file.txt".to_string(),relative_path: "fake-wide/a/file.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "b".to_string(),relative_path: "fake-wide/b".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "b".to_string(),is_sym: false,},
                TreeLeaf {name: "file.txt".to_string(),relative_path: "fake-wide/b/file.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "c".to_string(),relative_path: "fake-wide/c".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "c".to_string(),is_sym: false,},
                TreeLeaf {name: "file.txt".to_string(),relative_path: "fake-wide/c/file.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "file.md".to_string(),relative_path: "fake-wide/file.md".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.md".to_string(),is_sym: false,},
            ],
            paths_searched: 4,
        };
        let crawl_results = crawl::crawl_directory(&ARGS);
        assert_eq!(crawl_results.unwrap(), expected_crawl_results);
        test_dir.clean()
    }

    #[test]
    /// Produces crawl results equivalent to the below directory tree:
    /// 
    /// ```shell
    /// fake-search
    /// ├── b1
    /// │   ╰── f1.txt ...and should return: 123abc
    /// ├── b2
    /// │   ╰── f1.txt 789 Should match and re...
    /// ╰── b3
    ///     ╰── x1.txt 123def should match and re...
    ///
    ///3 matches, 6 searched
    /// ```
    /// 
    /// Testing functionality of `[--ignore | -I]` and `[--include | -X]` pattern filtering options.
    /// Includes ANSI color commands to highlight matching portions contained in window snippets.
    pub fn test_crawl_directory_search_contents() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-search";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", ROOT_TEST_DIR, "-x", "*.txt", "-I", "b4", r"\b\d{3}\b|\b\d{3}[a-zA-Z]{3,}\b", "-r", "20"]));
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("f1.txt", no_contents)?;
        test_dir.generate(".hidden.txt", Some("Matches but is hidden and should not return: 456qrs"))?;
        test_dir.generate("b1/f1.txt", Some("Matches and should return: 123xyz"))?;
        test_dir.generate("b1/f2.txt", Some("Should not match: def456ghi"))?;
        test_dir.generate("b2/f1.txt", Some("789 Should match and return"))?;
        test_dir.generate("b2/f2.txt", Some("Should not match since no digits"))?;
        test_dir.generate("b3/x1.txt", Some("123def should match and return"))?;
        test_dir.generate("b3/x2.md",  Some("456wrongext should match but wont return due to wrong extension"))?;
        test_dir.generate("b4/i1.txt", Some("123wrongdir should match but wont return due to ignored dir"))?;
        test_dir.generate("b4/i2.txt", Some("123wrongdir should match but wont return due to ignored dir"))?;
        let expected_crawl_results = CrawlResults {
            paths: vec![
                TreeLeaf {name: "f1.txt".to_string(),relative_path: "fake-search/b1/f1.txt".to_string(),is_dir: false,last_modified: None,size: None,window: Some("\u{1b}[38;5;248m...\u{1b}[0m\u{1b}[38;5;248mand should return: \u{1b}[0m\u{1b}[1m\u{1b}[38;5;42m123xyz\u{1b}[0m\u{1b}[38;5;248m\u{1b}[0m".to_string(),),display: "f1.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "f1.txt".to_string(),relative_path: "fake-search/b2/f1.txt".to_string(),is_dir: false,last_modified: None,size: None,window: Some("\u{1b}[38;5;248m\u{1b}[0m\u{1b}[1m\u{1b}[38;5;42m789\u{1b}[0m\u{1b}[38;5;248m Should match and re\u{1b}[0m\u{1b}[38;5;248m...\u{1b}[0m".to_string(),),display: "f1.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "x1.txt".to_string(),relative_path: "fake-search/b3/x1.txt".to_string(),is_dir: false,last_modified: None,size: None,window: Some("\u{1b}[38;5;248m\u{1b}[0m\u{1b}[1m\u{1b}[38;5;42m123def\u{1b}[0m\u{1b}[38;5;248m should match and re\u{1b}[0m\u{1b}[38;5;248m...\u{1b}[0m".to_string(),),display: "x1.txt".to_string(),is_sym: false,},
            ],
            paths_searched: 6,
        };
        let crawl_results = crawl::crawl_directory(&ARGS);
        assert_eq!(crawl_results.unwrap(), expected_crawl_results);
        test_dir.clean()
    }

    #[test]
    /// Produces crawl results equivalent to the below directory tree:
    /// 
    /// ```shell
    /// fake-core
    /// ├── .hidden
    /// ╰── d1
    ///     ╰── d2
    ///         ╰── d3
    ///             ╰── not-hidden.txt
    ///
    ///3 directories, 2 files
    /// ```
    /// 
    /// Tests functionality of `[--all | -a]` option to toggle inclusion of hidden entries.
    pub fn test_crawl_directory_hidden() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-hidden";
        static ARGS_NOT_HIDDEN: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", ROOT_TEST_DIR]));
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.create_file(".hidden", no_contents)?;
        test_dir.generate("d1/not-hidden.txt", no_contents)?;
        let expected_crawl_results = CrawlResults {
            paths: vec![
                TreeLeaf {name: "d1".to_string(),relative_path: "fake-hidden/d1".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "d1".to_string(),is_sym: false,},
                TreeLeaf {name: "not-hidden.txt".to_string(),relative_path: "fake-hidden/d1/not-hidden.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "not-hidden.txt".to_string(),is_sym: false,},
            ],
            paths_searched: 1,
        };
        let crawl_results = crawl::crawl_directory(&ARGS_NOT_HIDDEN);
        assert_eq!(crawl_results.unwrap(), expected_crawl_results);

        static ARGS_ALL: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--all", ROOT_TEST_DIR]));
        let expected_crawl_results = CrawlResults {
            paths: vec![
                TreeLeaf {name: ".hidden".to_string(),relative_path: "fake-hidden/.hidden".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: ".hidden".to_string(),is_sym: false,},
                TreeLeaf {name: "d1".to_string(),relative_path: "fake-hidden/d1".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "d1".to_string(),is_sym: false,},
                TreeLeaf {name: "not-hidden.txt".to_string(),relative_path: "fake-hidden/d1/not-hidden.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "not-hidden.txt".to_string(),is_sym: false,},
            ],
            paths_searched: 2,
        };
        let crawl_results = crawl::crawl_directory(&ARGS_ALL);
        assert_eq!(crawl_results.unwrap(), expected_crawl_results);
        test_dir.clean()
    }    



    #[test]
    /// Produces crawl results equivalent to the below directory tree:
    /// 
    /// ```shell
    /// fake-depth
    /// ├── d1
    /// │   ╰── d2
    /// │       ├── d3
    /// │       ╰── depth-3.txt
    /// ╰── depth-1.txt
    ///
    ///3 directories, 2 files
    /// ```
    /// 
    /// Tests functionality of `[--max-depth | -L]` option to restrict maximum recursive search depth.
    pub fn test_crawl_directory_max_depth() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-depth";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--max-depth", "3", ROOT_TEST_DIR]));
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("depth-1.txt", no_contents)?;
        test_dir.generate("d1/d2/depth-3.txt", no_contents)?;
        test_dir.generate("d1/d2/d3/d4/d5/d6/depth-7.txt", no_contents)?;
        let expected_crawl_results = CrawlResults {
            paths: vec![
                TreeLeaf {name: "d1".to_string(),relative_path: "fake-depth/d1".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "d1".to_string(),is_sym: false,},
                TreeLeaf {name: "d2".to_string(),relative_path: "fake-depth/d1/d2".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "d2".to_string(),is_sym: false,},
                TreeLeaf {name: "d3".to_string(),relative_path: "fake-depth/d1/d2/d3".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "d3".to_string(),is_sym: false,},
                TreeLeaf {name: "depth-3.txt".to_string(),relative_path: "fake-depth/d1/d2/depth-3.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "depth-3.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "depth-1.txt".to_string(),relative_path: "fake-depth/depth-1.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "depth-1.txt".to_string(),is_sym: false,},
            ],
            paths_searched: 2,
        };
        let crawl_results = crawl::crawl_directory(&ARGS);
        assert_eq!(crawl_results.unwrap(), expected_crawl_results);
        // println!("{crawl_results:#?}");
        test_dir.clean()
    }        

    #[test]
    /// Produces crawl results equivalent to the below directory tree:
    /// 
    /// ```shell
    /// fake-gitignore
    /// ├── README.md
    /// ╰── src
    ///     ╰── main.rs
    ///
    ///1 directory, 2 files
    /// ```
    /// 
    /// Testing `[--no-gitignore | -g]` option which toggles usage of .gitignore file search patterns in results.
    pub fn test_crawl_directory_gitignore() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-gitignore";
        static USE_GITIGNORE_ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", ROOT_TEST_DIR]));

        let gitignore_contents = Some("target/\n*.d\nsecrets.txt");
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("README.md", no_contents)?;
        test_dir.create_file(".gitignore", gitignore_contents)?;
        test_dir.generate("target/t1/file.txt", no_contents)?;
        test_dir.generate("secrets.txt", no_contents)?;
        test_dir.generate("01234.d", no_contents)?;
        test_dir.generate("56789.d", no_contents)?;
        test_dir.generate("src/main.rs", no_contents)?;
        let expected_crawl_results = CrawlResults {
            paths: vec![
                TreeLeaf {name: "README.md".to_string(),relative_path: "fake-gitignore/README.md".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "README.md".to_string(),is_sym: false,},
                TreeLeaf {name: "src".to_string(),relative_path: "fake-gitignore/src".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "src".to_string(),is_sym: false,},
                TreeLeaf {name: "main.rs".to_string(),relative_path: "fake-gitignore/src/main.rs".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "main.rs".to_string(),is_sym: false,},
            ],
            paths_searched: 2,
        };
        let crawl_results = crawl::crawl_directory(&USE_GITIGNORE_ARGS);
        assert_eq!(crawl_results.unwrap(), expected_crawl_results);

        static NO_GITIGNORE_ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--no-gitignore", ROOT_TEST_DIR]));
        let expected_crawl_results = CrawlResults {
            paths: vec![
                TreeLeaf {name: "01234.d".to_string(),relative_path: "fake-gitignore/01234.d".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "01234.d".to_string(),is_sym: false,},
                TreeLeaf {name: "56789.d".to_string(),relative_path: "fake-gitignore/56789.d".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "56789.d".to_string(),is_sym: false,},
                TreeLeaf {name: "README.md".to_string(),relative_path: "fake-gitignore/README.md".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "README.md".to_string(),is_sym: false,},
                TreeLeaf {name: "secrets.txt".to_string(),relative_path: "fake-gitignore/secrets.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "secrets.txt".to_string(),is_sym: false,},
                TreeLeaf {name: "src".to_string(),relative_path: "fake-gitignore/src".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "src".to_string(),is_sym: false,},
                TreeLeaf {name: "main.rs".to_string(),relative_path: "fake-gitignore/src/main.rs".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "main.rs".to_string(),is_sym: false,},
                TreeLeaf {name: "target".to_string(),relative_path: "fake-gitignore/target".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "target".to_string(),is_sym: false,},
                TreeLeaf {name: "t1".to_string(),relative_path: "fake-gitignore/target/t1".to_string(),is_dir: true,last_modified: None,size: None,window: None,display: "t1".to_string(),is_sym: false,},
                TreeLeaf {name: "file.txt".to_string(),relative_path: "fake-gitignore/target/t1/file.txt".to_string(),is_dir: false,last_modified: None,size: None,window: None,display: "file.txt".to_string(),is_sym: false,},
            ],
            paths_searched: 6,
        };
        let crawl_results = crawl::crawl_directory(&NO_GITIGNORE_ARGS);
        assert_eq!(crawl_results.unwrap(), expected_crawl_results);
        test_dir.clean()
    }   

    #[test]
    /// Produces directory and tree equivalent to:
    /// 
    /// ```shell
    ///  fake-tree
    ///  ├── d1
    ///  │   ├── f1.txt
    ///  │   ╰── f2.txt
    ///  ├── d2
    ///  │   ├── f1.txt
    ///  │   ╰── f2.txt
    ///  ├── emptydir
    ///  ├── f1.txt
    ///  ╰── f2.txt
    ///
    /// 3 directories, 6 files
    /// ```
    /// 
    /// Testing functionality of `tree::build_tree_from_paths` and structure of the generated `Tree`.
    pub fn test_tree_building() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-tree";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", ROOT_TEST_DIR]));

        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.create_file(".hidden", no_contents)?;
        test_dir.generate("f1.txt", no_contents)?;
        test_dir.generate("f2.txt", no_contents)?;
        test_dir.generate("d1/f1.txt", no_contents)?;
        test_dir.generate("d1/f2.txt", no_contents)?;
        test_dir.generate("d2/f1.txt", no_contents)?;
        test_dir.generate("d2/f2.txt", no_contents)?;
        test_dir.create_directory("emptydir")?;
        let crawl_results = crawl::crawl_directory(&ARGS);
        let received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS);
        let expected_output = Tree { display: "fake-tree".to_string(), name: "fake-tree".to_string(), path: None, entry_type: EntryType::Directory, last_modified: None, size: None, window: None, fmt_width: None, children: generate_tree_map([("d1".to_string(), Tree { display: "d1".to_string(), name: "d1".to_string(), path: None, entry_type: EntryType::Directory, last_modified: None, size: None, window: None, fmt_width: None, children: generate_tree_map([("f1.txt".to_string(), Tree { display: "f1.txt".to_string(), name: "f1.txt".to_string(), path: Some(PathBuf::from("fake-tree/d1/f1.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("f2.txt".to_string(), Tree { display: "f2.txt".to_string(), name: "f2.txt".to_string(), path: Some(PathBuf::from("fake-tree/d1/f2.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]) }), ("d2".to_string(), Tree { display: "d2".to_string(), name: "d2".to_string(), path: None, entry_type: EntryType::Directory, last_modified: None, size: None, window: None, fmt_width: None, children: generate_tree_map([("f1.txt".to_string(), Tree 
        { display: "f1.txt".to_string(), name: "f1.txt".to_string(), path: Some(PathBuf::from("fake-tree/d2/f1.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("f2.txt".to_string(), Tree { display: "f2.txt".to_string(), name: "f2.txt".to_string(), path: Some(PathBuf::from("fake-tree/d2/f2.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]) }), ("emptydir".to_string(), Tree { display: "emptydir".to_string(), name: "emptydir".to_string(), path: None, entry_type: EntryType::Directory, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("f1.txt".to_string(), Tree { display: "f1.txt".to_string(), name: "f1.txt".to_string(), path: Some(PathBuf::from("fake-tree/f1.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("f2.txt".to_string(), Tree { display: "f2.txt".to_string(), name: "f2.txt".to_string(), path: Some(PathBuf::from("fake-tree/f2.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]) };
        assert_eq!(expected_output, received_output);
        test_dir.clean()
    }


    #[test]
    /// Produces directory and tree equivalent to:
    /// 
    /// ```shell
    ///  fake-sort-name
    ///  ├── 1.txt
    ///  ├── 3.txt
    ///  ├── 5.txt
    ///  ├── A
    ///  │   ├── 2.txt
    ///  │   ╰── 3.txt
    ///  ├── b
    ///  │   ├── a.txt
    ///  │   ╰── z.txt
    ///  ╰── z
    ///      ├── aa.txt
    ///      ╰── ab.txt
    /// 
    /// 3 directories, 9 files
    /// ```
    /// 
    /// Testing functionality of `[--sort | -B]` and `[--reverse | -Z]` sorting tree by name in ascending and descending order.
    pub fn test_tree_sort_by_name() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-sort-name";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--sort", "name", ROOT_TEST_DIR]));

        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.create_directory("A")?;
        test_dir.generate("3.txt", no_contents)?;
        test_dir.generate("1.txt", no_contents)?;
        test_dir.generate("a/2.txt", no_contents)?;
        test_dir.generate("a/3.txt", no_contents)?;
        test_dir.generate("b/a.txt", no_contents)?;
        test_dir.generate("b/z.txt", no_contents)?;
        test_dir.generate("5.txt", no_contents)?;
        test_dir.generate("z/aa.txt", no_contents)?;
        test_dir.generate("z/ab.txt", no_contents)?;
        let crawl_results = crawl::crawl_directory(&ARGS);
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS);
        received_output.children.sort_by(|_, a, _, b| (&ARGS.sort_by)(a, b));        
        let order_received: Vec<_> = received_output.children.clone().into_iter().collect();
        let order_expected = vec![
            ("1.txt".to_string(),Tree {display: "1.txt".to_string(),name: "1.txt".to_string(),path: Some(PathBuf::from("fake-sort-name/1.txt")),entry_type: EntryType::File,last_modified: None,size: None,window: None,fmt_width: None,children: TreeMap::default(),},),
            ("3.txt".to_string(),Tree {display: "3.txt".to_string(),name: "3.txt".to_string(),path: Some(PathBuf::from("fake-sort-name/3.txt")),entry_type: EntryType::File,last_modified: None,size: None,window: None,fmt_width: None,children: TreeMap::default(),},),
            ("5.txt".to_string(),Tree {display: "5.txt".to_string(),name: "5.txt".to_string(),path: Some(PathBuf::from("fake-sort-name/5.txt")),entry_type: EntryType::File,last_modified: None,size: None,window: None,fmt_width: None,children: TreeMap::default(),},),
            ("A".to_string(),Tree {display: "A".to_string(),name: "A".to_string(),path: None,entry_type: EntryType::Directory,last_modified: None,size: None,window: None,fmt_width: None,children: generate_tree_map([("2.txt".to_string(), Tree { display: "2.txt".to_string(), name: "2.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/A/2.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("3.txt".to_string(), Tree { display: "3.txt".to_string(), name: "3.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/A/3.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]),},),
            ("b".to_string(),Tree {display: "b".to_string(),name: "b".to_string(),path: None,entry_type: EntryType::Directory,last_modified: None,size: None,window: None,fmt_width: None,children: generate_tree_map([("a.txt".to_string(), Tree { display: "a.txt".to_string(), name: "a.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/b/a.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("z.txt".to_string(), Tree { display: "z.txt".to_string(), name: "z.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/b/z.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]),},),
            ("z".to_string(),Tree {display: "z".to_string(),name: "z".to_string(),path: None,entry_type: EntryType::Directory,last_modified: None,size: None,window: None,fmt_width: None,children: generate_tree_map([("aa.txt".to_string(), Tree { display: "aa.txt".to_string(), name: "aa.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/z/aa.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("ab.txt".to_string(), Tree { display: "ab.txt".to_string(), name: "ab.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/z/ab.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]),},),
        ];

        assert_eq!(order_expected, order_received);

        // Test `--reverse` sorting order
        static ARGS_REVERSED: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--sort", "name", "--reverse", ROOT_TEST_DIR]));
        let crawl_results = crawl::crawl_directory(&ARGS_REVERSED);
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_REVERSED);
        received_output.children.sort_by(|_, a, _, b| (&ARGS_REVERSED.sort_by)(a, b));        
        let order_received: Vec<_> = received_output.children.clone().into_iter().collect();

        let order_expected = [
            ("z".to_string(),Tree {display: "z".to_string(),name: "z".to_string(),path: None,entry_type: EntryType::Directory,last_modified: None,size: None,window: None,fmt_width: None,children: generate_tree_map([("aa.txt".to_string(), Tree { display: "aa.txt".to_string(), name: "aa.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/z/aa.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("ab.txt".to_string(), Tree { display: "ab.txt".to_string(), name: "ab.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/z/ab.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]),},),
            ("b".to_string(),Tree {display: "b".to_string(),name: "b".to_string(),path: None,entry_type: EntryType::Directory,last_modified: None,size: None,window: None,fmt_width: None,children: generate_tree_map([("a.txt".to_string(), Tree { display: "a.txt".to_string(), name: "a.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/b/a.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("z.txt".to_string(), Tree { display: "z.txt".to_string(), name: "z.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/b/z.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]),},),
            ("A".to_string(),Tree {display: "A".to_string(),name: "A".to_string(),path: None,entry_type: EntryType::Directory,last_modified: None,size: None,window: None,fmt_width: None,children: generate_tree_map([("2.txt".to_string(), Tree { display: "2.txt".to_string(), name: "2.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/A/2.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("3.txt".to_string(), Tree { display: "3.txt".to_string(), name: "3.txt".to_string(), path: Some(PathBuf::from("fake-sort-name/A/3.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })]),},),
            ("5.txt".to_string(),Tree {display: "5.txt".to_string(),name: "5.txt".to_string(),path: Some(PathBuf::from("fake-sort-name/5.txt")),entry_type: EntryType::File,last_modified: None,size: None,window: None,fmt_width: None,children: TreeMap::default(),},),
            ("3.txt".to_string(),Tree {display: "3.txt".to_string(),name: "3.txt".to_string(),path: Some(PathBuf::from("fake-sort-name/3.txt")),entry_type: EntryType::File,last_modified: None,size: None,window: None,fmt_width: None,children: TreeMap::default(),},),
            ("1.txt".to_string(),Tree {display: "1.txt".to_string(),name: "1.txt".to_string(),path: Some(PathBuf::from("fake-sort-name/1.txt")),entry_type: EntryType::File,last_modified: None,size: None,window: None,fmt_width: None,children: TreeMap::default(),},),
        ];
        assert_eq!(order_received, order_expected);
        test_dir.clean()
    }    

    
    #[test]
    /// Produces directory and tree for running `rippy fake-sort-size --sort size --size` to generate:
    /// 
    /// ```shell
    ///  fake-sort-size
    ///  ├── (1.0 B) small.txt
    ///  ├── (3.0 B) medium.txt
    ///  ╰── (5.0 B) large.txt
    /// 
    /// 0 directories, 3 files
    /// ```
    /// 
    /// Testing functionality of `[--sort | -B]` and `[--reverse | -Z]` sorting tree by size in ascending and descending order.
    pub fn test_tree_sort_by_size() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-sort-size";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--sort", "size", "-s", ROOT_TEST_DIR]));

        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("small.txt", Some("1"))?;
        test_dir.generate("medium.txt", Some("111"))?;
        test_dir.generate("large.txt", Some("11111"))?;
        let crawl_results = crawl::crawl_directory(&ARGS);
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS);
        received_output.children.sort_by(|_, a, _, b| (&ARGS.sort_by)(a, b));     
        let order_received: Vec<_> = received_output.children.clone().into_iter().collect();
        let order_expected = vec![("small.txt".to_string(), Tree { display: "small.txt".to_string(), name: "small.txt".to_string(), path: Some(PathBuf::from("fake-sort-size/small.txt")), entry_type: EntryType::File, last_modified: None, size: Some(1), window: None, fmt_width: None, children: TreeMap::default() }), ("medium.txt".to_string(), Tree { display: "medium.txt".to_string(), name: "medium.txt".to_string(), path: Some(PathBuf::from("fake-sort-size/medium.txt")), entry_type: EntryType::File, last_modified: None, size: Some(3), window: None, fmt_width: None, children: TreeMap::default() }), ("large.txt".to_string(), Tree { display: "large.txt".to_string(), name: "large.txt".to_string(), path: Some(PathBuf::from("fake-sort-size/large.txt")), entry_type: EntryType::File, last_modified: None, size: Some(5), window: None, fmt_width: None, children: TreeMap::default() })];
        assert_eq!(order_expected, order_received);
        
        // Test `--reverse` sorting order
        static ARGS_REVERSED: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--sort", "size", "--reverse", "-s", ROOT_TEST_DIR]));
        let crawl_results = crawl::crawl_directory(&ARGS_REVERSED);
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_REVERSED);
        received_output.children.sort_by(|_, a, _, b| (&ARGS_REVERSED.sort_by)(a, b));        
        let order_received: Vec<_> = received_output.children.clone().into_iter().collect();
        let order_expected = vec![("large.txt".to_string(), Tree { display: "large.txt".to_string(), name: "large.txt".to_string(), path: Some(PathBuf::from("fake-sort-size/large.txt")), entry_type: EntryType::File, last_modified: None, size: Some(5), window: None, fmt_width: None, children: TreeMap::default() }), ("medium.txt".to_string(), Tree { display: "medium.txt".to_string(), name: "medium.txt".to_string(), path: Some(PathBuf::from("fake-sort-size/medium.txt")), entry_type: EntryType::File, last_modified: None, size: Some(3), window: None, fmt_width: None, children: TreeMap::default() }), ("small.txt".to_string(), Tree { display: "small.txt".to_string(), name: "small.txt".to_string(), path: Some(PathBuf::from("fake-sort-size/small.txt")), entry_type: EntryType::File, last_modified: None, size: Some(1), window: None, fmt_width: None, children: TreeMap::default() })];
        assert_eq!(order_received, order_expected);
        test_dir.clean()
    }    

    #[test]
    /// Produces directory and tree for running `rippy fake-sort-type --sort type` to generate:
    /// 
    /// ```shell
    ///  fake-sort-type
    ///  ├── d1
    ///  ├── d2
    ///  ├── f1.txt
    ///  ╰── f2.txt
    /// 
    /// 2 directories, 2 files
    /// ```
    /// 
    /// Testing functionality of `[--sort | -B]` and `[--reverse | -Z]` sorting tree by type in ascending and descending order.
    pub fn test_tree_sort_by_type() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-sort-type";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--sort", "type", ROOT_TEST_DIR]));

        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.create_file("f1.txt", no_contents)?;
        test_dir.create_file("f2.txt", no_contents)?;
        test_dir.create_directory("d1")?;
        test_dir.create_directory("d2")?;
        let crawl_results = crawl::crawl_directory(&ARGS);
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS);
        received_output.children.sort_by(|_, a, _, b| (&ARGS.sort_by)(a, b));     
        let order_received: Vec<_> = received_output.children.clone().into_iter().collect();
        
        let order_expected = vec![("d1".to_string(), Tree { display: "d1".to_string(), name: "d1".to_string(), path: None, entry_type: EntryType::Directory, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("d2".to_string(), Tree { display: "d2".to_string(), name: "d2".to_string(), path: None, entry_type: EntryType::Directory, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("f1.txt".to_string(), Tree { display: "f1.txt".to_string(), name: "f1.txt".to_string(), path: Some(PathBuf::from("fake-sort-type/f1.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("f2.txt".to_string(), Tree { display: "f2.txt".to_string(), name: "f2.txt".to_string(), path: Some(PathBuf::from("fake-sort-type/f2.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })];
        assert_eq!(order_expected, order_received);
        
        // Test `--reverse` sorting order
        static ARGS_REVERSED: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--sort", "type", "--reverse", ROOT_TEST_DIR]));
        let crawl_results = crawl::crawl_directory(&ARGS_REVERSED);
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_REVERSED);
        received_output.children.sort_by(|_, a, _, b| (&ARGS_REVERSED.sort_by)(a, b));        
        let order_received: Vec<_> = received_output.children.clone().into_iter().collect();

        let order_expected = vec![("f1.txt".to_string(), Tree { display: "f1.txt".to_string(), name: "f1.txt".to_string(), path: Some(PathBuf::from("fake-sort-type/f1.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("f2.txt".to_string(), Tree { display: "f2.txt".to_string(), name: "f2.txt".to_string(), path: Some(PathBuf::from("fake-sort-type/f2.txt")), entry_type: EntryType::File, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("d1".to_string(), Tree { display: "d1".to_string(), name: "d1".to_string(), path: None, entry_type: EntryType::Directory, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() }), ("d2".to_string(), Tree { display: "d2".to_string(), name: "d2".to_string(), path: None, entry_type: EntryType::Directory, last_modified: None, size: None, window: None, fmt_width: None, children: TreeMap::default() })];
        assert_eq!(order_received, order_expected);
        test_dir.clean()
    }        


    #[test]
    /// Produces directory and tree for running `rippy fake-sort-type --sort date --date` to generate:
    /// 
    /// ```shell
    ///  fake-sort-date
    ///  ├── (2024-11-18 19:58:45) time-0.txt
    ///  ├── (2024-11-18 19:58:45) time-1.txt
    ///  ├── (2024-11-18 19:58:45) time-2.txt
    ///  ╰── (2024-11-18 19:58:45) time-3.txt
    /// 
    /// 0 directories, 4 files
    /// ```
    /// 
    /// Testing functionality of `[--sort | -B]` and `[--reverse | -Z]` sorting tree by date in ascending and descending order.
    pub fn test_tree_sort_by_date() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-sort-date";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--sort", "date", "--date", ROOT_TEST_DIR]));

        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.create_file("time-0.txt", no_contents)?;
        thread::sleep(Duration::from_millis(1));
        test_dir.create_file("time-1.txt", no_contents)?;
        thread::sleep(Duration::from_millis(1));
        test_dir.create_file("time-2.txt", no_contents)?;
        thread::sleep(Duration::from_millis(1));
        test_dir.create_file("time-3.txt", no_contents)?;
        let crawl_results = crawl::crawl_directory(&ARGS);
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS);
        received_output.children.sort_by(|_, a, _, b| (&ARGS.sort_by)(a, b));     
        let order_received: Vec<_> = received_output.children.keys().collect();
        
        let mut order_expected = vec!["time-0.txt", "time-1.txt", "time-2.txt", "time-3.txt"];
        assert_eq!(order_expected, order_received);
        
        // Test `--reverse` sorting order
        static ARGS_REVERSED: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--sort", "date", "--date", "--reverse", ROOT_TEST_DIR]));
        let crawl_results = crawl::crawl_directory(&ARGS_REVERSED);
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_REVERSED);
        received_output.children.sort_by(|_, a, _, b| (&ARGS_REVERSED.sort_by)(a, b));        
        let order_received: Vec<_> = received_output.children.keys().collect();
        order_expected.reverse();
        assert_eq!(order_received, order_expected);
        test_dir.clean()
    }      

    #[test]
    /// Runs `rippy fake-paths --relative-path` on test directory to generate:
    /// 
    /// ```shell
    ///  fake-paths
    ///  ├── fake-paths/a
    ///  │   ├── fake-paths/a/f1.txt
    ///  │   ╰── fake-paths/a/x
    ///  │       ╰── fake-paths/a/x/f1.txt
    ///  ╰── fake-paths/b
    ///      ├── fake-paths/b/f1.txt
    ///      ╰── fake-paths/b/x
    ///          ╰── fake-paths/b/x/f1.txt
    /// 
    /// 4 directories, 4 files
    /// ```
    /// 
    /// Testing functionality of `[--relative-path | -P]` and `[--full-path | -K]` displaying paths for entries.
    pub fn test_tree_display_pathing() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-paths";
        static ARGS_RELATIVE: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--relative-path", ROOT_TEST_DIR]));
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("a/f1.txt", no_contents)?;
        test_dir.generate("a/x/f1.txt", no_contents)?;
        test_dir.generate("b/f1.txt", no_contents)?;
        test_dir.generate("b/x/f1.txt", no_contents)?;
        let crawl_results = crawl::crawl_directory(&ARGS_RELATIVE); 
        let received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_RELATIVE);
        let received_output: Vec<_> = received_output.iter().map(|child| (child.name.clone(), child.display.clone())).collect();
        let received_output = received_output.iter().map(|(k,v)| (k.as_str(), v.as_str())).collect::<Vec<(&str, &str)>>();
        let expected_output = vec![("fake-paths", "fake-paths"), ("a", "fake-paths/a"), ("f1.txt", "fake-paths/a/f1.txt"), ("x", "fake-paths/a/x"), ("f1.txt", "fake-paths/a/x/f1.txt"), ("b", "fake-paths/b"), ("f1.txt", "fake-paths/b/f1.txt"), ("x", "fake-paths/b/x"), ("f1.txt", "fake-paths/b/x/f1.txt")];
        assert_eq!(received_output, expected_output);
        
        // Absolute paths
        let cwd = std::env::current_dir().unwrap();
        static ARGS_FULL: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--full-path", ROOT_TEST_DIR]));
        let crawl_results = crawl::crawl_directory(&ARGS_FULL); 
        let received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_FULL);
        let received_output: Vec<_> = received_output.iter().map(|child| (child.name.clone(), child.display.clone())).collect();
        let received_output = received_output.iter().map(|(k,v)| (k.to_owned(), v.to_owned())).collect::<Vec<(String, String)>>();
        let expected_output = vec![
            (cwd.join("fake-paths").to_string_lossy().replace("\\", "/"), cwd.join("fake-paths").to_string_lossy().replace("\\", "/")),
            ("a".to_string(), cwd.join("fake-paths/a").to_string_lossy().replace("\\", "/")),
            ("f1.txt".to_string(), cwd.join("fake-paths/a/f1.txt").to_string_lossy().replace("\\", "/")),
            ("x".to_string(), cwd.join("fake-paths/a/x").to_string_lossy().replace("\\", "/")),
            ("f1.txt".to_string(), cwd.join("fake-paths/a/x/f1.txt").to_string_lossy().replace("\\", "/")),
            ("b".to_string(), cwd.join("fake-paths/b").to_string_lossy().replace("\\", "/")),
            ("f1.txt".to_string(), cwd.join("fake-paths/b/f1.txt").to_string_lossy().replace("\\", "/")), 
            ("x".to_string(), cwd.join("fake-paths/b/x").to_string_lossy().replace("\\", "/")), 
            ("f1.txt".to_string(), cwd.join("fake-paths/b/x/f1.txt").to_string_lossy().replace("\\", "/"))
            ];
        assert_eq!(received_output, expected_output);
        test_dir.clean()
    }

    #[test]
    /// Runs `rippy fake-fmt-width --window-radius 10 "X"` on test directory to generate:
    /// 
    /// ```shell
    ///  fake-fmt-width
    ///  ╰── docs
    ///      ├── short.txt               ...1---------X---------1...
    ///      ╰── very-long-file-name.txt ...1---------X---------1...
    /// 
    /// 2 matches, 3 searched
    /// ```
    /// 
    /// Testing calculations for matched snippet windows and their format widths from the entries.
    pub fn test_window_and_fmt_width() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-fmt-width";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--window-radius", "10", ROOT_TEST_DIR, "X"]));
        let target_contents: Option<&str> = Some("5---------4---------3---------2---------1---------X---------1---------2---------3---------4---------5");
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("docs/empty.txt", no_contents)?;
        test_dir.generate("docs/short.txt", target_contents)?;
        test_dir.generate("docs/very-long-file-name.txt", target_contents)?;
        let crawl_results = crawl::crawl_directory(&ARGS); 
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS);    
        received_output.calculate_fmt_width();
        let expected_output = vec![
            ("fake-fmt-width".to_string(), None, None),
            ("docs".to_string(), None, None),
            ("short.txt".to_string(), Some(23),Some("\u{1b}[38;5;248m...\u{1b}[0m\u{1b}[38;5;248m1---------\u{1b}[0m\u{1b}[1m\u{1b}[38;5;42mX\u{1b}[0m\u{1b}[38;5;248m---------1\u{1b}[0m\u{1b}[38;5;248m...\u{1b}[0m".to_string())),
            ("very-long-file-name.txt".to_string(), Some(23),Some("\u{1b}[38;5;248m...\u{1b}[0m\u{1b}[38;5;248m1---------\u{1b}[0m\u{1b}[1m\u{1b}[38;5;42mX\u{1b}[0m\u{1b}[38;5;248m---------1\u{1b}[0m\u{1b}[38;5;248m...\u{1b}[0m".to_string())),
        ];
        let received_output: Vec<_> = received_output.iter().map(|tree| (tree.name.clone(), tree.fmt_width, tree.window.clone())).collect();
        assert_eq!(received_output, expected_output);

        // Test with smaller radius
        static ARGS_SMALLER_RADIUS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--window-radius", "0", ROOT_TEST_DIR, "X"]));
        let crawl_results = crawl::crawl_directory(&ARGS_SMALLER_RADIUS); 
        let mut received_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_SMALLER_RADIUS);    
        received_output.calculate_fmt_width();
        let expected_output = vec![
            ("fake-fmt-width".to_string(), None, None),
            ("docs".to_string(), None, None),
            ("short.txt".to_string(), Some(23),Some("\u{1b}[38;5;248m...\u{1b}[0m\u{1b}[38;5;248m\u{1b}[0m\u{1b}[1m\u{1b}[38;5;42mX\u{1b}[0m\u{1b}[38;5;248m\u{1b}[0m\u{1b}[38;5;248m...\u{1b}[0m".to_string())),
            ("very-long-file-name.txt".to_string(), Some(23),Some("\u{1b}[38;5;248m...\u{1b}[0m\u{1b}[38;5;248m\u{1b}[0m\u{1b}[1m\u{1b}[38;5;42mX\u{1b}[0m\u{1b}[38;5;248m\u{1b}[0m\u{1b}[38;5;248m...\u{1b}[0m".to_string())),
        ];
        let received_output: Vec<_> = received_output.iter().map(|tree| (tree.name.clone(), tree.fmt_width, tree.window.clone())).collect();
        assert_eq!(received_output, expected_output);
        test_dir.clean()
    }

    #[test]
    /// Runs `rippy fake-count --just-counts` on test directory to generate:
    /// 
    /// ```shell
    /// 7 directories, 15 files
    /// ```
    /// 
    /// Testing functionality of `tree::count_tree` on imbalanced input and empty directories.
    pub fn test_count_tree() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-count";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", ROOT_TEST_DIR, "--just-counts"]));        
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.create_directory("e1")?;
        test_dir.create_directory("e2")?;
        test_dir.create_directory("e3")?;
        test_dir.create_file("f1.txt", no_contents)?;
        test_dir.create_file("f2.txt", no_contents)?;
        test_dir.create_file("f3.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f1.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f2.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f3.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f4.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f5.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f6.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f7.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f8.txt", no_contents)?;
        test_dir.generate("a/b/c/d/f9.txt", no_contents)?;
        test_dir.create_file("a.txt", no_contents)?;
        test_dir.create_file("b.txt", no_contents)?;
        test_dir.create_file("c.txt", no_contents)?;
        let crawl_results = crawl::crawl_directory(&ARGS); 
        let tree_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS);
        let mut counts_received = tree::TreeCounts::new();
        tree::count_tree(&tree_output, &mut counts_received, true);
        assert_eq!(counts_received, tree::TreeCounts{ dir_count: 7, file_count: 15});
        test_dir.clean()
    }
    
    #[test]
    /// Runs `rippy fake-writer --reverse` in test directory to generate:
    /// 
    /// ```shell
    /// fake-writer
    /// ├── src
    /// │   ├── prog.rs
    /// │   ╰── mod.rs
    /// ├── dist
    /// │   ╰── prog.exe
    /// ├── README.md
    /// ├── LICENSE
    /// ├── Cargo.toml
    /// ╰── Cargo.lock
    /// 
    /// 2 directories, 7 files
    /// ```
    /// 
    /// Testing functionality of `[--gray | -G]` and `[--reverse | -z]` and `tree::write_tree_to_buf` for tree rendering.
    pub fn test_write_tree_to_buf() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-writer";
        static ARGS_COLORED: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--reverse", ROOT_TEST_DIR]));
        let no_contents: Option<&str> = None;
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("dist/prog.exe", no_contents)?;
        test_dir.generate("src/prog.rs", no_contents)?;
        test_dir.generate("src/mod.rs", no_contents)?;
        test_dir.create_file("LICENSE", no_contents)?;
        test_dir.create_file(".gitignore", Some(".gitignore \nnotes.txt \nbuild.rs"))?;
        test_dir.generate("README.md", no_contents)?;
        test_dir.generate("Cargo.toml", no_contents)?;
        test_dir.generate("Cargo.lock", no_contents)?;
        test_dir.generate("build.rs", no_contents)?;
        test_dir.generate("notes.txt", no_contents)?;
        let crawl_results = crawl::crawl_directory(&ARGS_COLORED); 
        let mut counts = tree::TreeCounts::new();
        let mut tree_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_COLORED);     
        let mut buf_output = Vec::new();
        {
            let mut writer = std::io::BufWriter::new(&mut buf_output);
            tree::write_tree_to_buf(&mut tree_output, "", 0, "", true, &ARGS_COLORED, &mut counts, &mut writer)?;
        }
        let output_expected = " \u{1b}[1m\u{1b}[38;5;220mfake-writer\u{1b}[0m\n \u{1b}[38;5;220m├── \u{1b}[0m\u{1b}[1m\u{1b}[38;5;80msrc\u{1b}[0m\n \u{1b}[38;5;220m│\u{1b}[0m\u{a0}\u{a0} \u{1b}[38;5;80m├── \u{1b}[0mprog.rs\n \u{1b}[38;5;220m│\u{1b}[0m\u{a0}\u{a0} \u{1b}[38;5;80m╰── \u{1b}[0mmod.rs\n \u{1b}[38;5;220m├── \u{1b}[0m\u{1b}[1m\u{1b}[38;5;80mdist\u{1b}[0m\n \u{1b}[38;5;220m│\u{1b}[0m\u{a0}\u{a0} \u{1b}[38;5;80m╰── \u{1b}[0m\u{1b}[38;5;211mprog.exe\u{1b}[0m\n \u{1b}[38;5;220m├── \u{1b}[0mREADME.md\n \u{1b}[38;5;220m├── \u{1b}[0mLICENSE\n \u{1b}[38;5;220m├── \u{1b}[0mCargo.toml\n \u{1b}[38;5;220m╰── \u{1b}[0mCargo.lock\n\n";
        let output_received = String::from_utf8(buf_output).unwrap();
        assert_eq!(output_received, output_expected);
        assert_eq!(counts, tree::TreeCounts{ dir_count: 2, file_count: 7});

        // Same test but modify color, sort and gitignore options to test representation changes
        static ARGS_NO_COLOR: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--gray", "--no-gitignore", ROOT_TEST_DIR]));
        let crawl_results = crawl::crawl_directory(&ARGS_NO_COLOR); 
        let mut counts = tree::TreeCounts::new();
        let mut tree_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS_NO_COLOR);     
        let mut buf_output = Vec::new();
        {
            let mut writer = std::io::BufWriter::new(&mut buf_output);
            tree::write_tree_to_buf(&mut tree_output, "", 0, "", true, &ARGS_NO_COLOR, &mut counts, &mut writer)?;
        }
        let output_expected = " fake-writer\n ├── Cargo.lock\n ├── Cargo.toml\n ├── LICENSE\n ├── README.md\n ├── build.rs\n ├── dist\n │\u{a0}\u{a0} ╰── prog.exe\n ├── notes.txt\n ╰── src\n \u{a0}\u{a0}  ├── mod.rs\n \u{a0}\u{a0}  ╰── prog.rs\n\n";
        let output_received = String::from_utf8(buf_output).unwrap();
        assert_eq!(output_received, output_expected);
        assert_eq!(counts, tree::TreeCounts{ dir_count: 2, file_count: 9});
        test_dir.clean()
    }

    #[test]
    /// Runs `rippy fake-json --output fake-output.json --size` on test directory to generate:
    /// 
    /// ```shell
    ///  fake-json
    ///  ├── (566 B) Cargo.lock
    ///  ├── (224 B) Cargo.toml
    ///  ├── (250 B) LICENSE
    ///  ├── ( 78 B) README.md
    ///  ├── dist
    ///  │   ╰── (370 B) prog.exe
    ///  ╰── src
    ///      ├── (400 B) mod.rs
    ///      ╰── (150 B) prog.rs
    /// 
    /// 2 directories, 7 files
    /// ```
    /// 
    /// Testing functionality of `[--output <FILENAME>]` to validate JSON output of generated tree with arbitrary content for sizes.
    pub fn test_write_tree_to_json() -> Result<(), DirError> {
        const ROOT_TEST_DIR: &'static str = "fake-json";
        const JSON_FILE: &'static str = "fake-json/fake-output.json";
        static ARGS: LazyLock<rippy::args::RippyArgs> = LazyLock::new(|| generate_args_from(vec!["rippy", "--output", JSON_FILE, ROOT_TEST_DIR]));
        let test_dir = RootDirectory::new(ROOT_TEST_DIR);
        test_dir.generate("dist/prog.exe", Some("X".repeat(370)))?;
        test_dir.generate("src/prog.rs", Some("X".repeat(150)))?;
        test_dir.generate("src/mod.rs", Some("X".repeat(400)))?;
        test_dir.create_file("LICENSE", Some("X".repeat(250)))?;
        test_dir.generate("README.md", Some("X".repeat(78)))?;
        test_dir.generate("Cargo.toml", Some("X".repeat(224)))?;
        test_dir.generate("Cargo.lock", Some("X".repeat(566)))?;
        let crawl_results = crawl::crawl_directory(&ARGS); 
        let tree_output = tree::build_tree_from_paths(crawl_results.unwrap().paths, &ARGS);     
        tree_output.write_to_json_file(&ARGS)?;

        // Read the file back and deserialize
        let file_content = std::fs::read_to_string(&ARGS.output).unwrap();
        let json_received: serde_json::Value = serde_json::from_str(&file_content).unwrap();
    
        assert_eq!(json_received, json!({
            "name": "fake-json",
            "entry_type": "Directory",
            "last_modified": null,
            "size": null,
            "window": null,
            "children": [
              {
                "name": "Cargo.lock",
                "entry_type": "File",
                "last_modified": null,
                "size": null,
                "window": null,
                "children": []
              },
              {
                "name": "Cargo.toml",
                "entry_type": "File",
                "last_modified": null,
                "size": null,
                "window": null,
                "children": []
              },
              {
                "name": "dist",
                "entry_type": "Directory",
                "last_modified": null,
                "size": null,
                "window": null,
                "children": [
                  {
                    "name": "prog.exe",
                    "entry_type": "File",
                    "last_modified": null,
                    "size": null,
                    "window": null,
                    "children": []
                  }
                ]
              },
              {
                "name": "LICENSE",
                "entry_type": "File",
                "last_modified": null,
                "size": null,
                "window": null,
                "children": []
              },
              {
                "name": "README.md",
                "entry_type": "File",
                "last_modified": null,
                "size": null,
                "window": null,
                "children": []
              },
              {
                "name": "src",
                "entry_type": "Directory",
                "last_modified": null,
                "size": null,
                "window": null,
                "children": [
                  {
                    "name": "mod.rs",
                    "entry_type": "File",
                    "last_modified": null,
                    "size": null,
                    "window": null,
                    "children": []
                  },
                  {
                    "name": "prog.rs",
                    "entry_type": "File",
                    "last_modified": null,
                    "size": null,
                    "window": null,
                    "children": []
                  }
                ]
              }
            ]
          }));
        test_dir.clean()
    }
}