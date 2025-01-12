use std::collections::HashSet;

use hloo::index::{Candidates, SearchResultItem};

// 7 7 6 6 6
hloo::init_lookup!(LookupUtil, 32, 5, 1, 32);

fn generate_data(n: usize) -> Vec<(Bits, i64)> {
    let mut data = Vec::new();
    for i in 0..n {
        let bits = Bits::new(data_gen::random());
        data.push((bits, i as i64));
    }
    data
}

fn flip_bits(mut bits: Bits, n: usize) -> Bits {
    for _ in 0..n {
        let pos = (data_gen::random::<f32>() * 31f32) as usize;
        let bit = (bits.data[0] & (1 << pos)) >> pos;
        if bit == 0 {
            bits.data[0] |= 1 << pos;
        } else {
            bits.data[0] &= !(1 << pos);
        }
    }
    bits
}

fn naive_search<K: BitContainer, V: Clone>(data: &[(K, V)], key: K, distance: u32) -> Vec<SearchResultItem<V>> {
    Candidates::new(key, data).scan(distance)
}

#[test]
fn mem_lookup_compiles_and_runs_without_errors() {
    let mut lookup = LookupUtil::create_mem_lookup::<i64>();
    let data = generate_data(1);
    let target = data[0].0;
    lookup.insert(&data).unwrap();
    let result = lookup.search_simple(&target, 3);
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
    let mut lookup = LookupUtil::create_memmap_lookup::<i64>(tmp_path.path()).unwrap();
    let data = generate_data(1);
    let target = data[0].0;
    lookup.insert(&data).expect("failed to insert into memmap index");
    let result = lookup.search_simple(&target, 3);
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
    let expected = naive_search(&data, target, 3).into_iter().collect::<HashSet<_>>();
    let result = lookup.search_simple(&target, 3);
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
    let mut lookup = LookupUtil::create_memmap_lookup::<i64>(tmp_path.path()).unwrap();
    let data = generate_data(10);
    let target = flip_bits(data[0].0, 3);
    lookup.insert(&data).unwrap();
    let expected = naive_search(&data, target, 3).into_iter().collect::<HashSet<_>>();
    let result = lookup.search_simple(&target, 3);
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
    let result = lookup.search_simple(&target, 0);
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

    let lookup_map = {
        let tmp_path = tempfile::tempdir().unwrap();
        let mut lookup_map = LookupUtil::create_memmap_lookup::<i64>(tmp_path.path()).unwrap();
        lookup_map.insert(&data).unwrap();
        lookup_map
    };

    let expected = naive_search(&data, target, 3).into_iter().collect::<HashSet<_>>();

    let result_mem = lookup_mem.search_simple(&target, 3);
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

    let result_map = lookup_map.search_simple(&target, 3);
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

#[test]
fn memmap_lookup_can_be_saved_and_loaded() {
    let tmp_path = tempfile::tempdir().unwrap();
    let data = generate_data(10);
    let target = flip_bits(data[0].0, 3);
    let expected = naive_search(&data, target, 5).into_iter().collect::<HashSet<_>>();

    {
        let mut lookup = LookupUtil::create_memmap_lookup::<i64>(tmp_path.path()).unwrap();
        lookup.insert(&data).unwrap();
        let result = lookup.search_simple(&target, 3);
        assert_eq!(
            result.len(),
            expected.len(),
            "incorrect number of search results after load! expected {}, got {}",
            expected.len(),
            result.len()
        );
        for el in result {
            assert!(expected.contains(&el), "expected item is missing after load: {:?}", el);
        }
        lookup.persist().unwrap();
    }

    {
        let lookup = LookupUtil::load_memmap_lookup::<i64>(tmp_path.path()).unwrap();
        let result = lookup.search_simple(&target, 3);
        assert_eq!(
            result.len(),
            expected.len(),
            "incorrect number of search results after load! expected {}, got {}",
            expected.len(),
            result.len()
        );
        for el in result {
            assert!(expected.contains(&el), "expected item is missing after load: {:?}", el);
        }
    }
}
