macro_rules! cprint {
    ($fg:expr, $($arg:tt)*) => {{
        use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};
        use std::io::Write;
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some($fg)));
        let _ = write!(&mut stdout, $($arg)*);
        let _ = stdout.reset();
    }};
    (bold $fg:expr, $($arg:tt)*) => {{
        use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};
        use std::io::Write;
        let mut stdout = StandardStream::stdout(ColorChoice::Always);
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some($fg)).set_bold(true));
        let _ = write!(&mut stdout, $($arg)*);
        let _ = stdout.reset();
    }};
}

macro_rules! cprintln {
    ($fg:expr, $fmt:expr) => (cprint!($fg, concat!($fmt, "\n")));
    ($fg:expr, $fmt:expr, $($arg:tt)*) => (cprint!($fg, concat!($fmt, "\n"), $($arg)*));
    (bold $fg:expr, $fmt:expr) => (cprint!(bold $fg, concat!($fmt, "\n")));
    (bold $fg:expr, $fmt:expr, $($arg:tt)*) => (cprint!(bold $fg, concat!($fmt, "\n"), $($arg)*));
}
