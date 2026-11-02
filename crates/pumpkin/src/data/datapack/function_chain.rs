//! Bounds synchronous function execution, including recursive `execute run` calls.
use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;

const MAX_DEPTH: usize = 256;
const MAX_COMMANDS: usize = 65_536;
const DEPTH_ERROR: &str = "ArkBug: limite de profondeur des fonctions atteinte.";
const BUDGET_ERROR: &str = "ArkBug: limite de commandes des fonctions atteinte.";

#[derive(Clone, Copy, Default)]
struct State {
    depth: usize,
    commands: usize,
    failure: Option<&'static str>,
}
thread_local! {
    static CHAIN: Cell<State> = const { Cell::new(State {depth:0,commands:0,failure:None}) };
}

/// A thread-bound guard: nested commands must share their caller's budget.
pub(super) struct Guard(PhantomData<Rc<()>>);
impl Guard {
    pub(super) fn enter() -> Result<Self, &'static str> {
        CHAIN.with(|cell| {
            let mut state = cell.get();
            if let Some(reason) = state.failure {
                return Err(reason);
            }
            if state.depth >= MAX_DEPTH {
                state.failure = Some(DEPTH_ERROR);
                cell.set(state);
                return Err(DEPTH_ERROR);
            }
            state.depth += 1;
            cell.set(state);
            Ok(Self(PhantomData))
        })
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        CHAIN.with(|cell| {
            let mut state = cell.get();
            state.depth -= 1;
            cell.set(if state.depth == 0 {
                State::default()
            } else {
                state
            });
        });
    }
}

pub(crate) fn active() -> bool {
    CHAIN.with(|cell| cell.get().depth > 0)
}
pub(super) fn failure() -> Option<&'static str> {
    CHAIN.with(|cell| cell.get().failure)
}
pub(super) fn dispatch_next() -> bool {
    CHAIN.with(|cell| {
        let mut state = cell.get();
        // Direct visitor calls outside execution (e.g. resolution tests) are unbounded.
        if state.depth == 0 {
            return true;
        }
        if state.failure.is_some() {
            return false;
        }
        if state.commands >= MAX_COMMANDS {
            state.failure = Some(BUDGET_ERROR);
            cell.set(state);
            return false;
        }
        state.commands += 1;
        cell.set(state);
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn infinite_recursion_stops_and_next_chain_recovers() {
        fn call() -> Result<(), &'static str> {
            let _guard = Guard::enter()?;
            call()
        }
        assert_eq!(call(), Err(DEPTH_ERROR));
        assert!(!active());
        assert!(failure().is_none());
        let _guard = Guard::enter().unwrap();
        assert!(dispatch_next());
    }
    #[test]
    fn wide_and_nested_work_share_one_budget() {
        let guard = Guard::enter().unwrap();
        for _ in 0..MAX_COMMANDS / 2 {
            assert!(dispatch_next());
        }
        {
            let _nested = Guard::enter().unwrap();
            for _ in 0..MAX_COMMANDS / 2 {
                assert!(dispatch_next());
            }
        }
        assert!(!dispatch_next());
        assert_eq!(failure(), Some(BUDGET_ERROR));
        drop(guard);
        let _fresh = Guard::enter().unwrap();
        assert!(dispatch_next());
    }
    #[test]
    fn recursion_failure_aborts_sibling_work() {
        let _root = Guard::enter().unwrap();
        let mut nested = Vec::new();
        for _ in 1..MAX_DEPTH {
            nested.push(Guard::enter().unwrap());
        }
        assert!(Guard::enter().is_err());
        drop(nested);
        assert!(!dispatch_next());
    }
    #[test]
    fn unwinding_releases_the_chain() {
        assert!(
            std::panic::catch_unwind(|| {
                let _g = Guard::enter().unwrap();
                panic!("test");
            })
            .is_err()
        );
        assert!(!active());
        let _g = Guard::enter().unwrap();
        assert!(dispatch_next());
    }
    #[test]
    fn threads_have_independent_chains() {
        let _g = Guard::enter().unwrap();
        for _ in 0..MAX_COMMANDS {
            assert!(dispatch_next());
        }
        assert!(!dispatch_next());
        std::thread::spawn(|| {
            let _g = Guard::enter().unwrap();
            assert!(dispatch_next());
        })
        .join()
        .unwrap();
        assert_eq!(failure(), Some(BUDGET_ERROR));
    }
}
