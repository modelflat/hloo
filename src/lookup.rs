use std::marker::PhantomData;

use crate::index::{BitPermuter, Index, SearchResultItem};

pub struct Lookup<K, V, M, P, I> {
    indexes: Vec<I>,
    _dummy: PhantomData<(K, V, M, P)>,
}

impl<K, V, M, P, I> Lookup<K, V, M, P, I>
where
    K: Copy + Ord,
    M: Copy + Ord,
    P: BitPermuter<K, M>,
    I: Index<K, V, M, P>,
{
    pub fn new(indexes: Vec<I>) -> Self {
        Self {
            indexes,
            _dummy: Default::default(),
        }
    }

    pub fn indexes(&self) -> &[I] {
        &self.indexes
    }

    pub fn insert(&mut self, items: &[(K, V)]) -> Result<(), I::Error> {
        for index in &mut self.indexes {
            index.insert(items)?;
        }
        Ok(())
    }

    pub fn search(&self, key: K, distance: u32) -> Result<impl Iterator<Item = SearchResultItem<V>>, I::Error> {
        let mut result = Vec::with_capacity(self.indexes.len());
        for index in &self.indexes {
            let index_result = index.search(key, distance)?;
            result.push(index_result);
        }
        Ok(result.into_iter().flatten())
    }
}
