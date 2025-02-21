//! Labels for identifying named output scores, flows, and actions.

use alloc::boxed::Box;

use bevy_ecs::{define_label, intern::Interned};

pub use bevy_ecs::label::DynEq;

define_label!(
    /// Types that identify named output scores.
    ScoreLabel,
    SCORE_LABEL_INTERNER
);

/// A shorthand for `Interned<dyn ScoreLabel>`.
pub type InternedScoreLabel = Interned<dyn ScoreLabel>;

define_label!(
    /// Types that identify named [`Flow`](crate::flow::Flow)s.
    FlowLabel,
    FLOW_LABEL_INTERNER
);

/// A shorthand for `Interned<dyn FlowLabel>`.
pub type InternedFlowLabel = Interned<dyn FlowLabel>;

define_label!(
    /// Types that identify named actions.
    ActionLabel,
    ACTION_LABEL_INTERNER
);

/// A shorthand for `Interned<dyn ActionLabel>`.
pub type InternedActionLabel = Interned<dyn ActionLabel>;
