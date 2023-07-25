use std::collections::HashSet;

use hloo::create_permuter;

create_permuter!(Permuter, 32, 5, 2, 32);

#[test]
fn memory_lookup_compiles_and_runs_without_errors() {
    let mut lookup = Permuter::create_in_memory_lookup::<i64>();
    let target = Bits::new(rand::random());
    lookup
        .insert(&[
            (target, 0),
            (Bits::new(rand::random()), 1),
            (Bits::new(rand::random()), 2),
        ])
        .unwrap();
    let result = lookup.search(target, 5).unwrap().collect::<HashSet<_>>();
    assert_eq!(result.len(), 1, "incorrect number of search results!");
    assert_eq!(
        result.into_iter().next().map(|it| *it.data()),
        Some(0),
        "incorrect search result!"
    );
}

#[test]
fn memmap_lookup_compiles_and_runs_without_errors() {
    let tmp_path = tempfile::tempdir().unwrap();
    let mut lookup = Permuter::create_mem_map_lookup::<i64>(tmp_path.path()).unwrap();
    let target = Bits::new(rand::random());
    lookup
        .insert(&[
            (target, 0),
            (Bits::new(rand::random()), 1),
            (Bits::new(rand::random()), 2),
        ])
        .expect("failed to insert into memmap index");
    let result = lookup
        .search(target, 5)
        .expect("failed to search memmap index")
        .collect::<HashSet<_>>();
    assert_eq!(result.len(), 1, "incorrect number of search results!");
    assert_eq!(
        result.into_iter().next().map(|it| *it.data()),
        Some(0),
        "incorrect search result!"
    );
}
