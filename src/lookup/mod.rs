pub mod lookup_impl;

use std::{collections::HashSet, hash::Hash, marker::PhantomData, path::Path};

use hloo_core::BitContainer;

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
    pub fn flat_iter(&self) -> impl Iterator<Item = &SearchResultItem<V>> {
        self.result.iter().flatten()
    }

    pub fn into_flat_iter(self) -> impl Iterator<Item = SearchResultItem<V>> {
        self.result.into_iter().flatten()
    }
}

pub type IndexResult<T, K, V, M, I> = Result<T, <I as Index<K, V, M>>::Error>;

pub trait Lookup<K, V, M>
where
    K: BitContainer + Ord,
    V: Clone,
    M: Ord,
{
    type Index: Index<K, V, M>;

    fn indexes(&self) -> &[Self::Index];

    fn indexes_mut(&mut self) -> &mut [Self::Index];

    fn max_search_distance(&self) -> u32 {
        self.indexes()[0].permuter().n_blocks() - 1
    }

    /// Insert items into this lookup.
    fn insert(&mut self, items: &[(K, V)]) -> IndexResult<(), K, V, M, Self::Index> {
        for index in self.indexes_mut() {
            index.insert(items)?;
            index.refresh();
        }
        Ok(())
    }

    /// Remove items from the lookup by keys.
    fn remove(&mut self, keys: &[K]) -> IndexResult<(), K, V, M, Self::Index> {
        for index in self.indexes_mut() {
            index.remove(keys)?;
            index.refresh();
        }
        Ok(())
    }

    /// Perform a distance search.
    fn search(&self, key: &K, distance: u32) -> Result<SearchResult<V>, SearchError> {
        let max_distance = self.max_search_distance();
        if distance > max_distance {
            return Err(SearchError::DistanceExceedsMax {
                distance,
                max: max_distance,
            });
        }
        let mut candidates_scanned = 0usize;
        let mut result: Vec<Vec<SearchResultItem<V>>> = Vec::with_capacity(self.indexes().len());
        for index in self.indexes() {
            let candidates = index.get_candidates(key);
            candidates_scanned += candidates.len();
            result.push(candidates.scan(distance));
        }
        Ok(SearchResult {
            candidates_scanned,
            result,
        })
    }

    fn search_simple(&self, key: &K, distance: u32) -> HashSet<SearchResultItem<V>>
    where
        V: Hash + Eq,
    {
        self.search(key, distance)
            .expect("distance exceeds max")
            .into_flat_iter()
            .collect()
    }

    fn persist(&self) -> IndexResult<(), K, V, M, Self::Index>
    where
        Self::Index: PersistentIndex<K, M, Error = <Self::Index as Index<K, V, M>>::Error>,
    {
        for index in self.indexes() {
            index.persist()?;
        }
        Ok(())
    }
}

pub struct SimpleLookup<K, V, M, I> {
    indexes: Vec<I>,
    _dummy: PhantomData<(K, V, M)>,
}

impl<K, V, M, I> SimpleLookup<K, V, M, I> {
    pub fn new(indexes: Vec<I>) -> Self {
        Self {
            indexes,
            _dummy: Default::default(),
        }
    }
}

impl<K, V, M, I> SimpleLookup<K, V, M, I>
where
    K: BitContainer,
    V: Clone,
    M: Ord,
    I: Index<K, V, M> + PersistentIndex<K, M>,
{
    pub fn create(
        permuters: Vec<DynBitPermuter<K, M>>,
        sig: u64,
        path: &Path,
    ) -> Result<Self, <I as PersistentIndex<K, M>>::Error> {
        let mut indexes = Vec::new();
        for (i, p) in permuters.into_iter().enumerate() {
            let index_path = path.join(format!("index_{:04}_{:016x}.dat", i, sig));
            indexes.push(I::create(p, sig, &index_path)?)
        }
        Ok(Self::new(indexes))
    }

    pub fn load(
        permuters: Vec<DynBitPermuter<K, M>>,
        sig: u64,
        path: &Path,
    ) -> Result<Self, <I as PersistentIndex<K, M>>::Error> {
        let mut indexes = Vec::new();
        for (i, p) in permuters.into_iter().enumerate() {
            let index_path = path.join(format!("index_{:04}_{:016x}.dat", i, sig));
            indexes.push(I::load(p, sig, &index_path)?)
        }
        Ok(Self::new(indexes))
    }
}

impl<K, V, M, I> Lookup<K, V, M> for SimpleLookup<K, V, M, I>
where
    K: BitContainer + Ord,
    V: Clone,
    M: Ord,
    I: Index<K, V, M>,
{
    type Index = I;

    fn indexes(&self) -> &[Self::Index] {
        &self.indexes
    }

    fn indexes_mut(&mut self) -> &mut [Self::Index] {
        &mut self.indexes
    }
}
