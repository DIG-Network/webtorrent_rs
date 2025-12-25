// Ordering not currently used

/// Represents a piece selection range
#[derive(Debug, Clone)]
pub struct Selection {
    pub from: usize,
    pub to: usize,
    pub offset: usize,
    pub priority: i32,
    pub is_stream_selection: bool,
}

impl Selection {
    pub fn new(from: usize, to: usize, priority: i32, is_stream_selection: bool) -> Self {
        Self {
            from,
            to,
            offset: 0,
            priority,
            is_stream_selection,
        }
    }
}

/// Manages piece selections for a torrent
pub struct Selections {
    selections: Vec<Selection>,
}

impl Selections {
    pub fn new() -> Self {
        Self {
            selections: Vec::new(),
        }
    }

    pub fn insert(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.sort();
    }

    pub fn remove(&mut self, from: usize, to: usize, is_stream_selection: bool) {
        self.selections.retain(|s| {
            !(s.from == from && s.to == to && s.is_stream_selection == is_stream_selection)
        });
    }

    pub fn clear(&mut self) {
        self.selections.clear();
    }

    pub fn len(&self) -> usize {
        self.selections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&Selection> {
        self.selections.get(index)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Selection> {
        self.selections.iter()
    }

    pub fn sort(&mut self) {
        self.selections.sort_by(|a, b| {
            b.priority.cmp(&a.priority)
                .then_with(|| a.from.cmp(&b.from))
        });
    }

    pub fn swap(&mut self, i: usize, j: usize) {
        if i < self.selections.len() && j < self.selections.len() {
            self.selections.swap(i, j);
        }
    }
}

