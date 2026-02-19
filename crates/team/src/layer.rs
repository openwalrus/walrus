//! Team layer and depth tracking.

use crate::{SubAgent, WithTeam};
use ccore::{Agent, Layer};

tokio::task_local! {
    /// Current agent call depth in the nested call chain.
    pub(crate) static CALL_DEPTH: usize;
}

/// Maximum nesting depth for agent-as-tool calls.
pub(crate) const MAX_DEPTH: usize = 3;

/// Layer that adds a sub-agent as a tool to any agent.
///
/// # Example
///
/// ```rust,ignore
/// use cydonia_team::TeamLayer;
///
/// let agent = TeamLayer(analyst).layer(inner_agent);
/// ```
#[derive(Clone)]
pub struct TeamLayer<S: SubAgent>(pub S);

impl<A: Agent, S: SubAgent> Layer<A> for TeamLayer<S> {
    type Agent = WithTeam<A, S>;

    fn layer(self, agent: A) -> Self::Agent {
        WithTeam::new(agent, self.0)
    }
}
