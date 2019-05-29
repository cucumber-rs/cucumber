use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::panic;
use std::ops::Deref;

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

        let location = info.location()
                .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
                .unwrap_or_else(|| "Unknown location".to_owned());

        PanicDetails {
            payload,
            location,
        }
    }
}

#[derive(Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        Write::write(&mut *self.0.lock().unwrap(), data)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().flush()
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

    #[cfg(feature = "nightly")]
    fn run_quietly<F: FnOnce() -> T>(f: F) -> PanicTrap<T> {
        let stdout_sink = Arc::new(Mutex::new(vec![]));
        let stdout_sink_hook = stdout_sink.clone();
        let old_io = (
            io::set_print(Some(Box::new(Sink(stdout_sink.clone())))),
            io::set_panic(Some(Box::new(Sink(stdout_sink))))
        );

        let loud_panic_trap = PanicTrap::run_loudly(f);

        io::set_print(old_io.0);
        io::set_panic(old_io.1);

        let stdout = stdout_sink_hook.lock().expect("Stdout mutex poisoned").clone();
        PanicTrap {
            stdout,
            result: loud_panic_trap.result,
        }
    }

    #[cfg(not(feature = "nightly"))]
    fn run_quietly<F: FnOnce() -> T>(_f: F) -> PanicTrap<T> {
        panic!("PanicTrap cannot run quietly without the 'nightly' feature");
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

        let result = panic::catch_unwind(panic::AssertUnwindSafe(f))
            .map_err(|_| last_panic.lock().expect("Last panic mutex poisoned").clone().expect("Panic occurred but no panic details were set"));

        let _ = panic::take_hook();

        PanicTrap {
            stdout: vec![],
            result
        }
    }
}
