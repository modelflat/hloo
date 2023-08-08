use std::{collections::HashSet, hash::Hash, marker::PhantomData, path::Path};

use hloo_core::Distance;

use crate::{
    index::{Index, PersistentIndex, SearchResultItem},
    DynBitPermuter,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("distance ({distance}) exceeds maximum allowed distance for this candidate set ({max})")]
    DistanceExceedsMax { distance: u32, max: u32 },
}

pub struct SearchResult<V> {
    pub candidates_scanned: usize,
    pub result: Vec<Vec<SearchResultItem<V>>>,
}

impl<V> SearchResult<V> {
    pub fn iter(&self) -> impl Iterator<Item = &SearchResultItem<V>> {
        self.result.iter().flatten()
    }

    pub fn into_iter(self) -> impl Iterator<Item = SearchResultItem<V>> {
        self.result.into_iter().flatten()
    }
}

pub struct Lookup<K, V, M, I> {
    indexes: Vec<I>,
    _dummy: PhantomData<(K, V, M)>,
}

impl<K, V, M, I> Lookup<K, V, M, I> {
    pub fn new(indexes: Vec<I>) -> Self {
        Self {
            indexes,
            _dummy: Default::default(),
        }
    }

    pub fn indexes(&self) -> &[I] {
        &self.indexes
    }
}

impl<K, V, M, I> Lookup<K, V, M, I>
where
    K: Distance + Ord,
    V: Clone,
    M: Ord,
    I: Index<K, V, M>,
{
    pub fn max_search_distance(&self) -> u32 {
        self.indexes[0].permuter().n_blocks() - 1
    }

    /// Insert items into this lookup.
    pub fn insert(&mut self, items: &[(K, V)]) -> Result<(), I::Error> {
        for index in &mut self.indexes {
            index.insert(items)?;
            index.refresh();
        }
        Ok(())
    }

    /// Remove items from the lookup by keys.
    pub fn remove(&mut self, keys: &[K]) -> Result<(), I::Error> {
        for index in &mut self.indexes {
            index.remove(keys)?;
            index.refresh();
        }
        Ok(())
    }

    /// Perform a distance search.
    pub fn search(&self, key: &K, distance: u32) -> Result<SearchResult<V>, SearchError> {
        let max_distance = self.max_search_distance();
        if distance > max_distance {
            return Err(SearchError::DistanceExceedsMax {
                distance,
                max: max_distance,
            });
        }
        let mut candidates_scanned = 0usize;
        let mut result: Vec<Vec<SearchResultItem<V>>> = Vec::with_capacity(self.indexes.len());
        for index in &self.indexes {
            let candidates = index.get_candidates(key);
            candidates_scanned += candidates.len();
            result.push(candidates.scan(distance));
        }
        Ok(SearchResult {
            candidates_scanned,
            result,
        })
    }

    pub fn search_simple(&self, key: &K, distance: u32) -> HashSet<SearchResultItem<V>>
    where
        V: Hash + Eq,
    {
        self.search(key, distance)
            .expect("distance exceeds max")
            .into_iter()
            .collect()
    }
}

impl<K, V, M, I> Lookup<K, V, M, I>
where
    I: PersistentIndex<K, M>,
{
    pub fn create(permuters: Vec<DynBitPermuter<K, M>>, sig: u64, path: &Path) -> Result<Self, I::Error> {
        let mut indexes = Vec::new();
        for (i, p) in permuters.into_iter().enumerate() {
            let index_path = path.join(format!("index_{:04}_{:016x}.dat", i, sig));
            indexes.push(I::create(p, sig, &index_path)?)
        }
        Ok(Self::new(indexes))
    }

    pub fn load(permuters: Vec<DynBitPermuter<K, M>>, sig: u64, path: &Path) -> Result<Self, I::Error> {
        let mut indexes = Vec::new();
        for (i, p) in permuters.into_iter().enumerate() {
            let index_path = path.join(format!("index_{:04}_{:016x}.dat", i, sig));
            indexes.push(I::load(p, sig, &index_path)?)
        }
        Ok(Self::new(indexes))
    }

    pub fn persist(&self) -> Result<(), I::Error> {
        for index in &self.indexes {
            index.persist()?;
        }
        Ok(())
    }
}
