use std::io::Read;
use std::ops::Deref;
use std::panic::{self, UnwindSafe};
use std::sync::{Arc, Mutex};

use shh::{stderr, stdout};

#[derive(Clone)]
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
    pub async fn run(quiet: bool, f: impl futures::future::Future<Output = T> + UnwindSafe) -> PanicTrap<T> {
        if quiet {
            PanicTrap::run_quietly(f).await
        } else {
            PanicTrap::run_loudly(f).await
        }
    }

    async fn run_quietly(f: impl futures::future::Future<Output = T> + UnwindSafe) -> PanicTrap<T> {
        let mut stdout = stdout().expect("Failed to capture stdout");
        let mut stderr = stderr().expect("Failed to capture stderr");

        let mut trap = PanicTrap::run_loudly(f).await;

        stdout.read_to_end(&mut trap.stdout).unwrap();
        stderr.read_to_end(&mut trap.stderr).unwrap();

        trap
    }

    async fn run_loudly(f: impl futures::future::Future<Output = T> + UnwindSafe) -> PanicTrap<T> {
        use futures::future::FutureExt;
        let last_panic = Arc::new(Mutex::new(None));

        panic::set_hook({
            let last_panic = last_panic.clone();

            Box::new(move |info| {
                *last_panic.lock().expect("Last panic mutex poisoned") =
                    Some(PanicDetails::from_panic_info(info));
            })
        });

        let result = f.catch_unwind().await;

        let _ = panic::take_hook();

        let result = match last_panic.lock().expect("Last panic mutex poisoned").take() {
            Some(v) => Err(v),
            None => Ok(result.unwrap()),
        };

        PanicTrap {
            result: result,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
}
