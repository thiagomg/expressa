//! Execution availability only. Handles, workspaces and debugger state stay
//! with their existing owners; this model has no GTK or worker dependencies.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExecutionState {
    #[default]
    Idle,
    Preparing,
    Running,
    Uploading,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Prepare,
    PreparationFailed,
    Started,
    Finished,
    Uploaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unavailable {
    Preparing,
    Running,
    Uploading,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidTransition {
    pub state: ExecutionState,
    pub event: Event,
}

impl ExecutionState {
    /// Both another run and a project switch require the execution to be idle.
    pub fn availability(self) -> Result<(), Unavailable> {
        match self {
            Self::Idle => Ok(()),
            Self::Preparing => Err(Unavailable::Preparing),
            Self::Running => Err(Unavailable::Running),
            Self::Uploading => Err(Unavailable::Uploading),
        }
    }

    pub fn transition(&mut self, event: Event) -> Result<(), InvalidTransition> {
        *self = match (*self, event) {
            (Self::Idle, Event::Prepare) => Self::Preparing,
            (Self::Preparing, Event::PreparationFailed) => Self::Idle,
            (Self::Preparing, Event::Started) => Self::Running,
            (Self::Running, Event::Finished) => Self::Uploading,
            (Self::Uploading, Event::Uploaded) => Self::Idle,
            (state, event) => return Err(InvalidTransition { state, event }),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_through_a_run_and_failed_preparation() {
        let mut state = ExecutionState::default();
        assert_eq!(state.availability(), Ok(()));
        state.transition(Event::Prepare).unwrap();
        assert_eq!(state.availability(), Err(Unavailable::Preparing));
        state.transition(Event::PreparationFailed).unwrap();
        assert_eq!(state.availability(), Ok(()));
        state.transition(Event::Prepare).unwrap();
        state.transition(Event::Started).unwrap();
        assert_eq!(state.availability(), Err(Unavailable::Running));
        // Success, runtime error, stop and a disconnected worker all finish
        // the same way: the project stays unavailable until uploads finish.
        state.transition(Event::Finished).unwrap();
        assert_eq!(state.availability(), Err(Unavailable::Uploading));
        state.transition(Event::Uploaded).unwrap();
        assert_eq!(state.availability(), Ok(()));
    }

    #[test]
    fn rejects_overlapping_runs_and_out_of_order_completions() {
        for initial in [
            ExecutionState::Preparing,
            ExecutionState::Running,
            ExecutionState::Uploading,
        ] {
            let mut state = initial;
            assert_eq!(
                state.transition(Event::Prepare),
                Err(InvalidTransition {
                    state: initial,
                    event: Event::Prepare
                })
            );
            assert_eq!(state, initial);
        }
        let mut state = ExecutionState::Idle;
        for event in [
            Event::Started,
            Event::PreparationFailed,
            Event::Finished,
            Event::Uploaded,
        ] {
            assert!(state.transition(event).is_err());
            assert_eq!(state, ExecutionState::Idle);
        }
        state.transition(Event::Prepare).unwrap();
        assert!(state.transition(Event::Uploaded).is_err());
        assert_eq!(state, ExecutionState::Preparing);
    }
}
