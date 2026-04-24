use std::fmt;

use crate::tcolor::{ERROR_COLOR, WARN_COLOR};
use crate::{ansi_color, concat_str};

#[derive(Debug)]
pub enum RippyError {
    Cli(clap::Error),
    InvalidDirectory { path: String },
    InvalidRegex {
        context: &'static str,
        pattern: String,
        source: regex::Error,
    },
    InvalidPatternList {
        context: &'static str,
        pattern: String,
        source: regex::Error,
    },
    InvalidValue {
        flag: &'static str,
        value: String,
        reason: String,
    },
    SearchExpression(String),
    Walk(String),
    Io {
        context: &'static str,
        path: Option<String>,
        source: std::io::Error,
    },
    Json {
        path: String,
        source: serde_json::Error,
    },
}

impl RippyError {
    pub fn walk(message: impl Into<String>) -> Self {
        Self::Walk(message.into())
    }

    pub fn io(context: &'static str, path: Option<String>, source: std::io::Error) -> Self {
        Self::Io { context, path, source }
    }

    pub fn format_pretty(&self) -> String {
        if let Self::Cli(err) = self {
            return err.to_string();
        }

        let label = ansi_color!(ERROR_COLOR, bold=true, "Error");
        let body = match self {
            Self::Cli(_) => unreachable!(),
            Self::InvalidDirectory { path } => {
                let path_fmt = ansi_color!(WARN_COLOR, bold=false, path);
                concat_str!(
                    "The directory provided, '",
                    &path_fmt,
                    "', does not exist or is not a valid directory."
                )
            }
            Self::InvalidRegex {
                context,
                pattern,
                source,
            } => {
                let pattern_fmt = ansi_color!(WARN_COLOR, bold=false, pattern);
                let detail = indent_block(&source.to_string(), "  ");
                concat_str!(
                    "Invalid regular expression for ",
                    context,
                    ": ",
                    &pattern_fmt,
                    "\n",
                    &detail
                )
            }
            Self::InvalidPatternList {
                context,
                pattern,
                source,
            } => {
                let pattern_fmt = ansi_color!(WARN_COLOR, bold=false, pattern);
                let detail = indent_block(&source.to_string(), "  ");
                concat_str!(
                    "Invalid pattern generated for ",
                    context,
                    ": ",
                    &pattern_fmt,
                    "\n",
                    &detail
                )
            }
            Self::InvalidValue { flag, value, reason } => {
                let flag_fmt = ansi_color!(WARN_COLOR, bold=false, flag);
                let value_fmt = ansi_color!(WARN_COLOR, bold=false, value);
                concat_str!(
                    "Invalid value for ",
                    &flag_fmt,
                    ": ",
                    &value_fmt,
                    " (",
                    reason,
                    ")"
                )
            }
            Self::SearchExpression(message) => message.to_string(),
            Self::Walk(message) => concat_str!("Failed while traversing the directory tree: ", message),
            Self::Io {
                context,
                path,
                source,
            } => match path {
                Some(path) => {
                    let path_fmt = ansi_color!(WARN_COLOR, bold=false, path);
                    let source_text = source.to_string();
                    concat_str!(context, " [", &path_fmt, "]: ", &source_text)
                }
                None => {
                    let source_text = source.to_string();
                    concat_str!(context, ": ", &source_text)
                }
            },
            Self::Json { path, source } => {
                let path_fmt = ansi_color!(WARN_COLOR, bold=false, path);
                let source_text = source.to_string();
                concat_str!("Failed to write JSON output [", &path_fmt, "]: ", &source_text)
            }
        };

        concat_str!(&label, " ", &body)
    }
}

impl fmt::Display for RippyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cli(err) => write!(f, "{}", err),
            Self::InvalidDirectory { path } => {
                write!(f, "The directory '{}' does not exist or is not a valid directory.", path)
            }
            Self::InvalidRegex {
                context,
                pattern,
                source,
            } => write!(f, "Invalid regular expression for {} '{}': {}", context, pattern, source),
            Self::InvalidPatternList {
                context,
                pattern,
                source,
            } => write!(f, "Invalid pattern generated for {} '{}': {}", context, pattern, source),
            Self::InvalidValue { flag, value, reason } => {
                write!(f, "Invalid value for {} '{}': {}", flag, value, reason)
            }
            Self::SearchExpression(message) => write!(f, "{}", message),
            Self::Walk(message) => write!(f, "Failed while traversing the directory tree: {}", message),
            Self::Io {
                context,
                path,
                source,
            } => match path {
                Some(path) => write!(f, "{} [{}]: {}", context, path, source),
                None => write!(f, "{}: {}", context, source),
            },
            Self::Json { path, source } => {
                write!(f, "Failed to write JSON output [{}]: {}", path, source)
            }
        }
    }
}

impl std::error::Error for RippyError {}

fn indent_block(input: &str, prefix: &str) -> String {
    let mut output = String::new();
    for (index, line) in input.lines().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        output.push_str(prefix);
        output.push_str(line);
    }
    output
}