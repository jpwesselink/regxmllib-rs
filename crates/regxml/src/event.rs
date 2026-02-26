/// Severity level of a fragment builder event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Info,
    Warn,
    Error,
    Fatal,
}

/// Identifies the type of anomaly detected during fragment building.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCode {
    UnknownGroup,
    UnknownProperty,
    VersionByteMismatch,
    UnexpectedDefinition,
    CircularStrongReference,
    MissingRootObject,
    MissingPartitionPack,
    MissingPrimerPack,
    MalformedSet,
}

/// An event emitted by [`super::FragmentBuilder`] during processing.
#[derive(Debug, Clone)]
pub struct FragmentEvent {
    pub code: EventCode,
    pub severity: EventSeverity,
    pub reason: String,
    pub location: String,
}

/// Decision returned by an event handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventHandlerDecision {
    Continue,
    Abort,
}

/// Callback interface for fragment builder events.
pub trait EventHandler: Send + Sync {
    fn handle(&mut self, event: &FragmentEvent) -> EventHandlerDecision;
}
