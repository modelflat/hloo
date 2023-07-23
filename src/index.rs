use std::hash::Hash;

#[derive(Clone, Copy, Eq, Debug)]
pub struct SearchResultItem<V> {
    data: V,
    distance: u32,
}

impl<V> SearchResultItem<V> {
    pub fn new(data: V, distance: u32) -> Self {
        Self { data, distance }
    }
}

impl<V> PartialEq for SearchResultItem<V>
where
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<V> Hash for SearchResultItem<V>
where
    V: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

pub trait Index<K, V> {
    fn insert(&mut self, key: K, value: V);
    fn insert_many(&mut self, items: impl Iterator<Item = (K, V)>);
    fn search(&self, key: K) -> Vec<SearchResultItem<V>>;
}
