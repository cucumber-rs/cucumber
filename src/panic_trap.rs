use std::io::Read;
use std::ops::Deref;
use std::panic;
use std::sync::{Arc, Mutex};

use shh::{stderr, stdout};

#[derive(Debug, Clone)]
pub struct PanicDetails {
    pub payload: String,
    pub location: String,
}

impl PanicDetails {
    fn from_panic_info(info: &panic::PanicInfo) -> PanicDetails {
        let payload = if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.deref().to_owned()
        } else {
            "Opaque panic payload".to_owned()
        };

        let location = info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "Unknown panic location".to_owned());

        PanicDetails { payload, location }
    }
}

pub struct PanicTrap<T> {
    pub result: Result<T, PanicDetails>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl<T> PanicTrap<T> {
    pub fn run<F: FnOnce() -> T>(quiet: bool, f: F) -> PanicTrap<T> {
        if quiet {
            PanicTrap::run_quietly(f)
        } else {
            PanicTrap::run_loudly(f)
        }
    }

    fn run_quietly<F: FnOnce() -> T>(f: F) -> PanicTrap<T> {
        let mut stdout = stdout().expect("Failed to capture stdout");
        let mut stderr = stderr().expect("Failed to capture stderr");

        let mut trap = PanicTrap::run_loudly(f);

        stdout.read_to_end(&mut trap.stdout).unwrap();
        stderr.read_to_end(&mut trap.stderr).unwrap();

        trap
    }

    fn run_loudly<F: FnOnce() -> T>(f: F) -> PanicTrap<T> {
        let last_panic = Arc::new(Mutex::new(None));

        panic::set_hook({
            let last_panic = last_panic.clone();

            Box::new(move |info| {
                *last_panic.lock().expect("Last panic mutex poisoned") =
                    Some(PanicDetails::from_panic_info(info));
            })
        });

        let result = panic::catch_unwind(panic::AssertUnwindSafe(f));

        let _ = panic::take_hook();

        PanicTrap {
            result: result.map_err(|_| {
                last_panic
                    .lock()
                    .expect("Last panic mutex poisoned")
                    .take()
                    .expect("Panic occurred but no panic details were set")
            }),
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
}
