use std::{marker::PhantomData, path::Path};

use bit_permute::{BitPermuter, Distance};

use crate::index::{Index, PersistentIndex, SearchError, SearchResultItem};

pub struct Lookup<K, V, M, P, I> {
    indexes: Vec<I>,
    _dummy: PhantomData<(K, V, M, P)>,
}

impl<K, V, M, P, I> Lookup<K, V, M, P, I> {
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

impl<K, V, M, P, I> Lookup<K, V, M, P, I>
where
    K: Distance + Ord,
    V: Clone,
    M: Ord,
    P: BitPermuter<K, M>,
    I: Index<K, V, M, P>,
{
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
    pub fn search(
        &self,
        key: &K,
        distance: u32,
    ) -> Result<impl Iterator<Item = SearchResultItem<V>>, SearchError<I::Error>> {
        let mut result: Vec<Vec<SearchResultItem<V>>> = Vec::with_capacity(self.indexes.len());
        for index in &self.indexes {
            let index_result = index.search(key, distance)?;
            result.push(index_result);
        }
        Ok(result.into_iter().flatten())
    }
}

impl<K, V, M, P, I> Lookup<K, V, M, P, I>
where
    I: PersistentIndex<P>,
{
    pub fn create(permuters: Vec<P>, sig: u64, path: &Path) -> Result<Self, I::Error> {
        let mut indexes = Vec::new();
        for (i, p) in permuters.into_iter().enumerate() {
            let index_path = path.join(format!("index_{:04}_{:016x}.dat", i, sig));
            indexes.push(I::create(p, sig, &index_path)?)
        }
        Ok(Self::new(indexes))
    }

    pub fn load(permuters: Vec<P>, sig: u64, path: &Path) -> Result<Self, I::Error> {
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
