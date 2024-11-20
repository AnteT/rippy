use std::io;

// Windows ANSI terminal support flags (only defined on Windows)
#[cfg(windows)]
const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
#[cfg(windows)]
const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;

/* ========================= 8 bit ANSI color scheme ========================= */
pub const ERROR_COLOR: Option<&'static str> = Some("\x1b[38;5;203m");
pub const WARN_COLOR: Option<&'static str> = Some("\x1b[38;5;184m");
const ROOT_COLOR: Option<&'static str> = Some("\x1b[38;5;220m");
const DIR_COLOR: Option<&'static str> = Some("\x1b[38;5;80m");
const EXEC_COLOR: Option<&'static str> = Some("\x1b[38;5;211m"); 

// const FILE_COLOR: Option<&'static str> = Some("\x1b[38;5;252m"); // Originally
const FILE_COLOR: Option<&'static str> = None; // Revised 2024-09-19

const SYM_COLOR: Option<&'static str> = Some("\x1b[38;5;147m");
const DETAILS_COLOR: Option<&'static str> = Some("\x1b[38;5;248m");
const MATCHES_COLOR: Option<&'static str> = Some("\x1b[38;5;42m");
const SEARCH_COLOR: Option<&'static str> = Some("\x1b[38;5;220m");
const ZERO_COLOR: Option<&'static str> = Some("\x1b[38;5;220m");
const NONE_COLOR: Option<&'static str> = None;

#[cfg(windows)]
extern "system" {
    fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, lpMode: *mut u32) -> i32;
    fn SetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, dwMode: u32) -> i32;
}

/// Enable ANSI escape sequences if currently on Windows. Returns `true` if successful or unnecessary (i.e., not Windows) or `false` if enabling ANSI support on Windows failed.
pub fn enable_ansi_support() -> bool {
    if cfg!(windows) {
        match enable_windows_ansi_support() {
            Ok(()) => true,
            Err(_) => false,
        }
    } else {
        // On non-Windows systems, no need to enable ANSI, so we return `true`.
        true
    }
}

#[cfg(not(windows))]
/// Dummy implementation for non-Windows platforms, required for compilation bounds checks.
fn enable_windows_ansi_support() -> io::Result<()> {
    Ok(())
}

/// SAFETY: If there is another way of doing this without adding external dependencies that also use unsafe, then I could not find it.
#[cfg(windows)]
fn enable_windows_ansi_support() -> io::Result<()> {
    unsafe {
        let std_out = GetStdHandle(STD_OUTPUT_HANDLE);
        if std_out.is_null() {
            return Err(io::Error::last_os_error());
        }

        let mut console_mode = 0;
        if GetConsoleMode(std_out, &mut console_mode) == 0 {
            return Err(io::Error::last_os_error());
        }

        if SetConsoleMode(std_out, console_mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct RippySchema {
    pub root: Option<&'static str>,
    pub dir: Option<&'static str>,
    pub exec: Option<&'static str>,
    pub file: Option<&'static str>,
    pub sym: Option<&'static str>,
    pub detail: Option<&'static str>,
    pub search: Option<&'static str>,
    pub window: Option<&'static str>,
    pub muted: Option<&'static str>,
    pub zero: Option<&'static str>,
}

impl RippySchema {
    /// Returns the color schema using the const assigned to each styling parameter based on search and grayscale arguments.
    pub fn get_color_schema(is_grayscale: bool) -> Self {
        if is_grayscale {
            RippySchema {
                root: NONE_COLOR,
                dir: NONE_COLOR,
                exec: NONE_COLOR,
                file: NONE_COLOR,
                sym: NONE_COLOR,
                detail: NONE_COLOR,
                search: NONE_COLOR,
                window: NONE_COLOR,
                muted: NONE_COLOR,
                zero: NONE_COLOR,
            }
        } else {
            RippySchema {
                root: ROOT_COLOR,
                dir: DIR_COLOR,
                exec: EXEC_COLOR,
                file: FILE_COLOR,
                sym: SYM_COLOR,
                detail: DETAILS_COLOR,
                search: SEARCH_COLOR,
                window: MATCHES_COLOR,
                muted: DETAILS_COLOR,
                zero: ZERO_COLOR,
            }
        }
    }
}

#[macro_export]
/// Formats and returns a String with the provided ANSI terminal styling commands using an optional keyword argument for bold.
macro_rules! ansi_color {
    ($color:expr, bold=$is_bold:expr, $text:expr) => {{
        let bold_fmt = if $is_bold { "\x1b[1m" } else { "" };
        match $color {
            Some(color_code) => {
                let mut result = String::with_capacity(bold_fmt.len() + $text.len() + 16); // Extra space for color (max len: 11) and reset codes (len: 4)
                result.push_str(bold_fmt);
                result.push_str(color_code);
                result.push_str($text.as_ref());
                result.push_str( "\x1b[0m" ); // Reset code (len: 4)
                result
            }
            None => {$text.to_string()} // direct return
        }
    }};
    ($color:expr, $text:expr) => {
        ansi_color!($color, bold=false, $text)
    };
}

#[macro_export]
/// Concatenates provided strings using push_str method to avoid overhead of format macro with explicit capacity bounds.
macro_rules! concat_str {
    ($($item:expr),*) => {{
        let total_length = 0 $( + $item.len() )*;
        let mut result = String::with_capacity(total_length);
        $( result.push_str($item.as_ref()); )*
        result
    }};
}