use std::collections::HashSet;

use hloo::init_lookup;

init_lookup!(LookupUtil, 32, 5, 1, 32);
// 7 7 6 6 6

fn generate_data(n: usize) -> Vec<(Bits, i64)> {
    let mut data = Vec::new();
    for i in 0..n {
        data.push((Bits::new(rand::random()), i as i64))
    }
    data
}

fn flip_bits(mut bits: Bits, n: usize) -> Bits {
    for _ in 0..n {
        let pos = (rand::random::<f32>() * 31f32) as usize;
        let bit = (bits.data[0] & (1 << pos)) >> pos;
        if bit == 0 {
            bits.data[0] = bits.data[0] | (1 << pos);
        } else {
            bits.data[0] = bits.data[0] & !(1 << pos);
        }
    }
    bits
}

#[test]
fn mem_lookup_compiles_and_runs_without_errors() {
    let mut lookup = LookupUtil::create_mem_lookup::<i64>();
    let data = generate_data(1);
    let target = data[0].0;
    lookup.insert(&data).unwrap();
    let result = lookup.search(&target, 3).unwrap().collect::<HashSet<_>>();
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
    let mut lookup = LookupUtil::create_memmap_lookup::<i64>(0, tmp_path.path()).unwrap();
    let data = generate_data(1);
    let target = data[0].0;
    lookup.insert(&data).expect("failed to insert into memmap index");
    let result = lookup
        .search(&target, 3)
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
fn mem_lookup_works_correctly() {
    let mut lookup = LookupUtil::create_mem_lookup::<i64>();
    let data = generate_data(10);
    let target = flip_bits(data[0].0, 3);
    lookup.insert(&data).unwrap();
    let expected = hloo::index::scan_block(&data, &target, 5)
        .into_iter()
        .collect::<HashSet<_>>();
    let result = lookup.search(&target, 3).unwrap().collect::<HashSet<_>>();
    println!("{:?}", expected);
    println!("{:?}", result);
    assert_eq!(
        result.len(),
        expected.len(),
        "incorrect number of search results! expected {}, got {}",
        expected.len(),
        result.len()
    );
    for el in result {
        assert!(expected.contains(&el), "expected item is missing: {:?}", el);
    }
}

#[test]
fn memmap_lookup_works_correctly() {
    let tmp_path = tempfile::tempdir().unwrap();
    let mut lookup = LookupUtil::create_memmap_lookup::<i64>(0, tmp_path.path()).unwrap();
    let data = generate_data(10);
    let target = flip_bits(data[0].0, 3);
    println!("init_data = {:?}", data);
    lookup.insert(&data).unwrap();
    let expected = hloo::index::scan_block(&data, &target, 5)
        .into_iter()
        .collect::<HashSet<_>>();
    let result = lookup.search(&target, 3).unwrap().collect::<HashSet<_>>();
    println!("{:?}", expected);
    println!("{:?}", result);
    assert_eq!(
        result.len(),
        expected.len(),
        "incorrect number of search results! expected {}, got {}",
        expected.len(),
        result.len()
    );
    for el in result {
        assert!(expected.contains(&el), "expected item is missing: {:?}", el);
    }
}

#[test]
fn mem_lookup_single_entry() {
    let init_data = vec![(Bits { data: [851899373] }, 0)];
    let target = init_data[0].0;
    let mut lookup = LookupUtil::create_mem_lookup::<i64>();
    lookup.insert(&init_data).unwrap();
    let result = lookup.search(&target, 0).unwrap().collect::<HashSet<_>>();
    assert_eq!(result.len(), 1, "incorrect number of search results!");
    assert_eq!(
        result.into_iter().next().map(|it| *it.data()),
        Some(0),
        "incorrect search result!"
    );
}

#[test]
fn naive_results_correspond_to_hloo() {
    let data = generate_data(1000);
    let target = flip_bits(data[0].0, 3);

    let mut lookup_mem = LookupUtil::create_mem_lookup::<i64>();
    lookup_mem.insert(&data).unwrap();
    let tmp_path = tempfile::tempdir().unwrap();
    let mut lookup_map = LookupUtil::create_memmap_lookup::<i64>(0, tmp_path.path()).unwrap();
    lookup_map.insert(&data).unwrap();

    let expected = hloo::index::scan_block(&data, &target, 3)
        .into_iter()
        .collect::<HashSet<_>>();

    let result_mem = lookup_mem.search(&target, 3).unwrap().collect::<HashSet<_>>();
    assert_eq!(
        result_mem.len(),
        expected.len(),
        "incorrect number of results! expected {}, got {}",
        expected.len(),
        result_mem.len()
    );
    for el in result_mem {
        assert!(expected.contains(&el), "expected item is missing: {:?}", el);
    }

    let result_map = lookup_map.search(&target, 3).unwrap().collect::<HashSet<_>>();
    assert_eq!(
        result_map.len(),
        expected.len(),
        "incorrect number of results! expected {}, got {}",
        expected.len(),
        result_map.len()
    );
    for el in result_map {
        assert!(expected.contains(&el), "expected item is missing: {:?}", el);
    }
}
