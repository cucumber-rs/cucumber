use std::io::{Read, Seek, SeekFrom};
use std::ops::Deref;
use std::panic;
use std::sync::{Arc, Mutex};

use gag::Redirect;
use tempfile::tempfile;

#[derive(Clone)]
pub struct PanicDetails {
    pub payload: String,
    pub location: String,
}

impl PanicDetails {
    fn from_panic_info(info: &panic::PanicInfo) -> PanicDetails {
        let info_payload = info.payload();
        let payload = if let Some(s) = info_payload.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = info_payload.downcast_ref::<&str>() {
            s.deref().to_owned()
        } else {
            "Opaque panic payload".to_owned()
        };

        let location = info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "Unknown location".to_owned());

        PanicDetails { payload, location }
    }
}

pub struct PanicTrap<T> {
    pub stdout: Vec<u8>,
    pub result: Result<T, PanicDetails>,
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
        let mut tmp = tempfile().unwrap();

        let loud_panic_trap = {
            let _stdout =
                Redirect::stdout(tmp.try_clone().unwrap()).expect("Failed to capture stdout");
            let _stderr =
                Redirect::stderr(tmp.try_clone().unwrap()).expect("Failed to capture stderr");

            PanicTrap::run_loudly(f)
        };

        let mut stdout = Vec::new();
        tmp.seek(SeekFrom::Start(0)).unwrap();
        tmp.read_to_end(&mut stdout).unwrap();

        PanicTrap {
            stdout,
            result: loud_panic_trap.result,
        }
    }

    fn run_loudly<F: FnOnce() -> T>(f: F) -> PanicTrap<T> {
        let last_panic = Arc::new(Mutex::new(None));

        panic::set_hook({
            let last_panic_hook = last_panic.clone();
            Box::new(move |info| {
                let mut state = last_panic_hook.lock().expect("last_panic unpoisoned");
                *state = Some(PanicDetails::from_panic_info(info));
            })
        });

        let result = panic::catch_unwind(panic::AssertUnwindSafe(f)).map_err(|_| {
            last_panic
                .lock()
                .expect("Last panic mutex poisoned")
                .clone()
                .expect("Panic occurred but no panic details were set")
        });

        let _ = panic::take_hook();

        PanicTrap {
            stdout: vec![],
            result,
        }
    }
}
