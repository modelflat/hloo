use std::collections::HashSet;

use hloo::create_permuter;

create_permuter!(Permuter, 32, 1, 1, 32);

fn generate_data(n: usize) -> Vec<(Bits, i64)> {
    let mut data = Vec::new();
    for i in 0..n {
        data.push((Bits::new(rand::random()), i as i64))
    }
    data
}

#[test]
fn memory_lookup_compiles_and_runs_without_errors() {
    let mut lookup = Permuter::create_in_memory_lookup::<i64>();
    let data = generate_data(1);
    let target = data[0].0;
    println!("init data = {:?}", data);
    lookup.insert(&data).unwrap();
    let result = lookup.search(target, 0).unwrap().collect::<HashSet<_>>();
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
    let data = generate_data(3);
    let target = data[0].0;
    lookup.insert(&data).expect("failed to insert into memmap index");
    let result = lookup
        .search(target, 0)
        .expect("failed to search memmap index")
        .collect::<HashSet<_>>();
    assert_eq!(result.len(), 1, "incorrect number of search results!");
    assert_eq!(
        result.into_iter().next().map(|it| *it.data()),
        Some(0),
        "incorrect search result!"
    );
}

#[test]
fn test_weird_error() {
    println!("{:032b}", 851899373);
    let init_data = vec![(Bits { data: [851899373] }, 0)];
    let target = init_data[0].0;
    let mut lookup = Permuter::create_in_memory_lookup::<i64>();
    lookup.insert(&init_data).unwrap();
    let result = lookup.search(target, 0).unwrap().collect::<HashSet<_>>();
    assert_eq!(result.len(), 1, "incorrect number of search results!");
    assert_eq!(
        result.into_iter().next().map(|it| *it.data()),
        Some(0),
        "incorrect search result!"
    );
}
