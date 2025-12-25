use webtorrent::selections::{Selections, Selection};

#[tokio::test]
async fn test_selections_new() {
    let selections = Selections::new();
    
    assert_eq!(selections.len(), 0);
    assert!(selections.is_empty());
}

#[tokio::test]
async fn test_selections_insert() {
    let mut selections = Selections::new();
    
    let selection = Selection::new(0, 10, 1, false);
    selections.insert(selection);
    
    assert_eq!(selections.len(), 1);
    assert!(!selections.is_empty());
}

#[tokio::test]
async fn test_selections_remove() {
    let mut selections = Selections::new();
    
    selections.insert(Selection::new(0, 10, 1, false));
    selections.insert(Selection::new(20, 30, 2, false));
    
    assert_eq!(selections.len(), 2);
    
    selections.remove(0, 10, false);
    
    assert_eq!(selections.len(), 1);
}

#[tokio::test]
async fn test_selections_sort() {
    let mut selections = Selections::new();
    
    selections.insert(Selection::new(0, 10, 1, false));
    selections.insert(Selection::new(20, 30, 3, false));
    selections.insert(Selection::new(40, 50, 2, false));
    
    // Should be sorted by priority (descending)
    assert_eq!(selections.get(0).unwrap().priority, 3);
    assert_eq!(selections.get(1).unwrap().priority, 2);
    assert_eq!(selections.get(2).unwrap().priority, 1);
}

#[tokio::test]
async fn test_selections_clear() {
    let mut selections = Selections::new();
    
    selections.insert(Selection::new(0, 10, 1, false));
    selections.insert(Selection::new(20, 30, 2, false));
    
    selections.clear();
    
    assert_eq!(selections.len(), 0);
    assert!(selections.is_empty());
}

#[tokio::test]
async fn test_selections_swap() {
    let mut selections = Selections::new();
    
    selections.insert(Selection::new(0, 10, 1, false));
    selections.insert(Selection::new(20, 30, 2, false));
    
    let first = selections.get(0).unwrap().priority;
    let second = selections.get(1).unwrap().priority;
    
    selections.swap(0, 1);
    
    assert_eq!(selections.get(0).unwrap().priority, second);
    assert_eq!(selections.get(1).unwrap().priority, first);
}

