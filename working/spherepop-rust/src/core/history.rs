use super::event::Event;
use super::event::Symbol;

/// History is the identity of a Spherepop computation.
/// It grows monotonically — events are never removed or reordered.
/// Observable state is derived FROM history, not stored alongside it.
#[derive(Debug, Clone, Default)]
pub struct History {
    events: Vec<Event>,
}

impl History {
    pub fn new() -> Self { Self { events: Vec::new() } }

    pub fn append(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[Event] { &self.events }
    pub fn len(&self) -> usize { self.events.len() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }

    pub fn meld(&mut self, other: &History) {
        self.events.extend(other.events.iter().cloned());
    }

    pub fn pop_count(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, Event::Pop(_))).count()
    }

    pub fn refuse_count(&self) -> usize {
        self.events.iter().filter(|e| matches!(e, Event::Refuse { .. })).count()
    }

    pub fn ever_popped(&self, s: &Symbol) -> bool {
        self.events.iter().any(|e| matches!(e, Event::Pop(x) if x == s))
    }

    pub fn ever_refused(&self, s: &Symbol) -> bool {
        self.events.iter().any(|e| matches!(e, Event::Refuse { target, .. } if target == s))
    }
}

impl std::fmt::Display for History {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, e) in self.events.iter().enumerate() {
            if i > 0 { write!(f, ", ")?; }
            write!(f, "{}", e)?;
        }
        write!(f, "]")
    }
}

impl PartialEq for History {
    fn eq(&self, other: &Self) -> bool { self.events == other.events }
}
impl Eq for History {}
