use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

static PANE_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A strongly-typed ID for a pane.
/// T serves as a marker to distinguish between different pane types at compile time,
/// or can be `()` for a generic handle.
pub struct PaneId<T: ?Sized = ()> {
    id: usize,
    _marker: PhantomData<T>,
}

impl<T: ?Sized> PaneId<T> {
    /// Creates a new unique PaneId
    pub fn new() -> Self {
        Self {
            id: PANE_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            _marker: PhantomData,
        }
    }

    /// Creates a PaneId from a raw usize. Use with caution.
    pub fn from_raw(id: usize) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    /// Returns the raw underlying ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Casts this ID to a different type or generic ID.
    pub fn cast<U: ?Sized>(self) -> PaneId<U> {
        PaneId {
            id: self.id,
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized> Clone for PaneId<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for PaneId<T> {}

impl<T: ?Sized> PartialEq for PaneId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: ?Sized> Eq for PaneId<T> {}

impl<T: ?Sized> Hash for PaneId<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: ?Sized> Debug for PaneId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PaneId({})", self.id)
    }
}

impl<T: ?Sized> Default for PaneId<T> {
    fn default() -> Self {
        Self::new()
    }
}
