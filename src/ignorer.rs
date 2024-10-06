use std::path::Path;
use ignore::gitignore::Gitignore;

#[derive(Clone, Debug, Default)]
/// Custom implementation to streamline usage of `ignore::gitignore::Gitignore` down to only the most basic functions required for `rippy`.
pub struct Ignorer {
    pub matcher: Option<Gitignore>
}
impl Ignorer {
    /// Creates a new `Ignorer` from a filepath to what is assumed to be a `.gitignore` like format containing globs to match or whitelist.
    pub fn new<P: AsRef<Path>>(gitignore_path: P) -> Self {
        Ignorer { matcher: Some(Gitignore::new(gitignore_path).0) }
    }
    /// Check if path should be ignored based on current `matcher` presence, value and whether path represents directory.
    pub fn is_ignore<P: AsRef<Path>>(&self, path: P, is_dir: bool) -> bool {
        self.matcher.as_ref().map_or_else(|| false, |m| m.matched(path, is_dir).is_ignore())
    }
    #[allow(unused)]
    /// Check if `matcher` has been initialized with a `Gitignore`.
    pub fn has_matcher(&self) -> bool {
        self.matcher.as_ref().is_some()
    }
}
impl<P: AsRef<Path>> From<P> for Ignorer {
    fn from(value: P) -> Self {
        Self::new(value)
    }
}