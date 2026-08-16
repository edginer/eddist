use std::sync::{Mutex, MutexGuard};

/// Held for the lifetime of every [`EnvGuard`], so tests that touch the process
/// environment never overlap with each other.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Sets environment variables and restores their previous values on drop.
/// Guards must not be nested: ENV_LOCK is not reentrant.
pub struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    /// `None` removes the variable for the duration of the guard.
    pub fn apply(vars: &[(&'static str, Option<&str>)]) -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let saved = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();
        set_all(vars);
        Self { saved, _lock: lock }
    }

    pub fn set(vars: &[(&'static str, &str)]) -> Self {
        let vars = vars.iter().map(|(k, v)| (*k, Some(*v))).collect::<Vec<_>>();
        Self::apply(&vars)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        let restore = self
            .saved
            .iter()
            .map(|(k, v)| (*k, v.as_deref()))
            .collect::<Vec<_>>();
        set_all(&restore);
    }
}

fn set_all(vars: &[(&'static str, Option<&str>)]) {
    // SAFETY: callers hold ENV_LOCK, and every environment mutation in tests goes
    // through EnvGuard, so no other thread reads or writes the environment here.
    unsafe {
        for (k, v) in vars {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_then_restores() {
        {
            let _guard = EnvGuard::set(&[("EDDIST_ENV_GUARD_PROBE", "inner")]);
            assert_eq!(std::env::var("EDDIST_ENV_GUARD_PROBE").unwrap(), "inner");
        }
        assert!(std::env::var("EDDIST_ENV_GUARD_PROBE").is_err());

        {
            let _guard = EnvGuard::apply(&[
                ("EDDIST_ENV_GUARD_PROBE", Some("again")),
                ("EDDIST_ENV_GUARD_ABSENT", None),
            ]);
            assert_eq!(std::env::var("EDDIST_ENV_GUARD_PROBE").unwrap(), "again");
            assert!(std::env::var("EDDIST_ENV_GUARD_ABSENT").is_err());
        }
        assert!(std::env::var("EDDIST_ENV_GUARD_PROBE").is_err());
    }
}
