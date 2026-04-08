/// Shared test utilities to prevent CWD races between test modules.
///
/// Both `config::tests` and `api::setup::tests` change the process working directory,
/// which is global state. This single mutex serializes all such tests across modules.

use std::sync::{Mutex, OnceLock};

pub fn cwd_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
