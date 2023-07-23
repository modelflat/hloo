use core::slice;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    marker::PhantomData,
    ops::{Index, Range},
    path::{Path, PathBuf},
};

use memmap2::MmapMut;

pub struct MmVec<T> {
    mmap: MmapMut,
    len: usize,
    path: PathBuf,
    dummy: PhantomData<T>,
}

impl<T> MmVec<T>
where
    T: Copy,
{
    /// Creates a vector from parts.
    fn from_parts(mmap: MmapMut, len: usize, path: PathBuf) -> Self {
        Self {
            mmap,
            len,
            path,
            dummy: PhantomData,
        }
    }

    /// # Safety
    /// This is unsafe because we can't guarantee that this file contains a vector of the correctly aligned type T.
    pub unsafe fn from_file(len: usize, path: PathBuf) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        let meta = file.metadata()?;
        let cap = (meta.len() / std::mem::size_of::<T>() as u64) as usize;
        assert!(len <= cap, "length should be <= capacity");

        let mmap = MmapMut::map_mut(&file)?;
        #[cfg(unix)]
        {
            mmap.advise(memmap2::Advice::Random).ok();
        }

        Ok(Self::from_parts(mmap, len, path))
    }

    /// Dumps a regular in-memory vector into a file, then mmaps it.
    pub fn from_vec(vec: Vec<T>, path: PathBuf) -> io::Result<Self> {
        let len = vec.len();
        if len == 0 {
            panic!("vec should not be empty");
        }

        vec_to_file(vec, &path)?;

        // SAFETY: this is safe because we just created this file, and know that it contains correct data.
        unsafe { Self::from_file(len, path) }
    }

    /// Creates an uninitialized vector with given capacity.
    pub fn with_length_uninitialized(len: usize, path: PathBuf) -> io::Result<Self> {
        {
            let file = File::create(&path)?;
            let n_bytes = len * std::mem::size_of::<T>();
            file.set_len(n_bytes as u64)?;
        }
        // SAFETY: we just created this file
        unsafe { Self::from_file(len, path) }
    }

    /// Copies self into path, and returns a vector MMapped to this path.
    pub fn copy_to(&self, path: PathBuf) -> io::Result<Self> {
        fs::copy(&self.path, &path)?;

        // SAFETY: we just created this file.
        unsafe { Self::from_file(self.len, path) }
    }

    /// Moves self into path, and returns re-mmapped vector to this path.
    pub fn move_to(self, path: PathBuf) -> io::Result<Self> {
        let len = self.len;
        fs::rename(&self.path, &path)?;
        drop(self);

        // SAFETY: we just created this file.
        unsafe { Self::from_file(len, path) }
    }

    /// Map the existing vector into a new one at `Path`, with indexes.
    pub fn map_with_index<O, F>(&self, f: F, path: PathBuf) -> io::Result<MmVec<O>>
    where
        F: Fn(usize, T) -> O,
        O: Copy,
    {
        let mut target = MmVec::with_length_uninitialized(self.len(), path)?;
        let target_slice = unsafe { target.as_slice_mut() };
        for (idx, el) in unsafe { self.as_slice() }.iter().enumerate() {
            target_slice[idx] = f(idx, *el);
        }
        Ok(target)
    }

    /// Map the existing vector into a new one at `Path`.
    #[inline]
    pub fn map<O, F>(&self, f: F, path: PathBuf) -> io::Result<MmVec<O>>
    where
        F: Fn(T) -> O,
        O: Copy,
    {
        self.map_with_index(|_, el| f(el), path)
    }

    /// Merge self and other into a single vector. Both vectors should be sorted!
    pub fn merge_sorted<O, F>(&self, other: &[T], sort_key: F, path: PathBuf) -> io::Result<Self>
    where
        F: Fn(T) -> O,
        O: Ord,
    {
        let new_size = self.len() + other.len();
        let mut merged = Self::with_length_uninitialized(new_size, path)?;

        let mut idx_left = 0;
        let mut idx_right = 0;
        let this = unsafe { self.as_slice() };
        for el in unsafe { merged.as_slice_mut() }.iter_mut() {
            if idx_left == self.len() {
                *el = other[idx_right];
                idx_right += 1;
            } else if idx_right == other.len() {
                *el = this[idx_left];
                idx_left += 1;
            } else {
                let left = this[idx_left];
                let right = other[idx_right];
                if sort_key(left) <= sort_key(right) {
                    *el = left;
                    idx_left += 1;
                } else {
                    *el = right;
                    idx_right += 1;
                }
            }
        }

        Ok(merged)
    }

    /// Returns a path to the backing file.
    #[inline]
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns length of the vector.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// # Safety
    /// This is unsafe because we can't guarantee that file has not been tampered with.
    #[inline]
    #[must_use]
    pub unsafe fn as_slice(&self) -> &[T] {
        slice::from_raw_parts(self.mmap.as_ptr() as *const T, self.len)
    }

    /// # Safety
    /// This is unsafe because we can't guarantee that file has not been tampered with.
    #[inline]
    #[must_use]
    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        slice::from_raw_parts_mut(self.mmap.as_mut_ptr() as *mut T, self.len)
    }

    pub fn sort_unstable_by_key<K, F>(&mut self, f: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        unsafe { self.as_slice_mut() }.sort_unstable_by_key(f);
    }

    pub fn binary_search_by_key<B, F>(&self, key: &B, f: F) -> Result<usize, usize>
    where
        F: FnMut(&T) -> B,
        B: Ord,
    {
        unsafe { self.as_slice() }.binary_search_by_key(key, f)
    }
}

impl<T> Index<usize> for MmVec<T>
where
    T: Copy,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &self.as_slice()[index] }
    }
}

impl<T> Index<Range<usize>> for MmVec<T>
where
    T: Copy,
{
    type Output = [T];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        unsafe { &self.as_slice()[index] }
    }
}

fn vec_to_file<T>(vec: Vec<T>, path: &Path) -> io::Result<()> {
    let len_bytes = vec.len() * std::mem::size_of::<T>();

    // SAFETY: this is safe because we know that the vector is valid - we own it
    let bytes = unsafe { slice::from_raw_parts(vec.as_ptr() as *const u8, len_bytes) };

    let mut file = File::create(path)?;
    file.write_all(bytes)
}
