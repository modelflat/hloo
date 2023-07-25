use std::marker::PhantomData;

use crate::{index::SearchResultItem, BitPermuter, Index};

pub struct Lookup<K, V, M, P, I> {
    indexes: Vec<I>,
    _dummy: PhantomData<(K, V, M, P)>,
}

impl<K, V, M, P, I> Lookup<K, V, M, P, I>
where
    K: Copy + Ord,
    P: BitPermuter<K, V, M>,
    I: Index<K, V, M, P>,
{
    pub fn new(indexes: Vec<I>) -> Self {
        Self {
            indexes,
            _dummy: Default::default(),
        }
    }

    pub fn insert(&mut self, items: &[(K, V)]) -> Result<(), I::Error> {
        for index in &mut self.indexes {
            index.insert(items)?;
        }
        Ok(())
    }

    pub fn search(&self, key: K, distance: u32) -> Result<Vec<SearchResultItem<V>>, I::Error> {
        let mut result = Vec::new();
        for index in &self.indexes {
            let index_result = index.search(key, distance)?;
            result.extend(index_result);
        }
        Ok(result)
    }
}
