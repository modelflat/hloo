use core::slice;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Seek, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
};

use memmap2::{MmapMut, MmapOptions};

use crate::util::merge_sorted;

pub struct MmVec<T> {
    data: Data<T>,
    path: PathBuf,
}

impl<T> MmVec<T>
where
    T: Copy,
{
    fn new(data: Data<T>, path: PathBuf) -> Self {
        Self { data, path }
    }

    /// Creates an uninitialized vector with given length.
    pub fn with_length_uninit(len: usize, path: PathBuf) -> io::Result<Self> {
        let file = create_new_file(&path)?;
        // SAFETY: we just created this file
        let data = unsafe { Data::new_uninit(file, len)? };
        Ok(Self::new(data, path))
    }

    /// Dumps a regular in-memory vector into path, then mmaps it.
    pub fn from_vec(vec: Vec<T>, path: PathBuf) -> io::Result<Self> {
        let file = create_new_file(&path)?;
        // SAFETY: we just created this file
        let data = unsafe { Data::new_with_data(file, &vec)? };
        Ok(Self::new(data, path))
    }

    /// Creates a vector from the given path.
    pub fn from_path(path: PathBuf) -> io::Result<Self> {
        let file = open_file(&path)?;
        // SAFETY: we are going to check the data
        let data = unsafe {
            let data = Data::<T>::from_file_unchecked(file)?;
            // TODO proper signature check depending on type T, and possibly CRC
            assert!(
                data.sig() == 0,
                "vector at {}: signature does not match requested type!",
                path.display()
            );
            // only whole-file, fully initialized vectors are supported
            assert!(
                data.len() == data.capacity() as u64,
                "vector at {}: has trailing data! len {}, cap {}",
                path.display(),
                data.len(),
                data.capacity(),
            );
            data
        };
        Ok(Self::new(data, path))
    }

    /// Returns a path to the backing file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns length of the vector.
    #[must_use]
    pub fn len(&self) -> usize {
        unsafe { self.data.len() as usize }
    }

    /// Whether this vector is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// # Safety
    /// This is unsafe because we can't guarantee that file has not been tampered with.
    #[must_use]
    pub unsafe fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }

    /// # Safety
    /// This is unsafe because we can't guarantee that file has not been tampered with.
    #[must_use]
    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        self.data.as_slice_mut()
    }

    /// Flushes memory-mapped data into file.
    pub fn flush(&self) -> io::Result<()> {
        self.data.flush()
    }

    /// Copies self into path, and returns a new vector at this path.
    pub fn copy_to(&self, path: PathBuf) -> io::Result<Self> {
        self.flush()?;
        fs::copy(&self.path, &path)?;

        let copied = open_file(&path)?;
        // SAFETY: we just created this file from a valid vector
        let copied_data = unsafe { Data::from_file_unchecked(copied)? };
        Ok(Self::new(copied_data, path))
    }

    /// Moves self into path, and returns a new vector at this path.
    pub fn move_to(self, path: PathBuf) -> io::Result<Self> {
        self.flush()?;
        fs::rename(&self.path, &path)?;
        drop(self);

        let moved = open_file(&path)?;
        // SAFETY: we just created this file from a valid vector
        let moved_data = unsafe { Data::from_file_unchecked(moved)? };
        Ok(Self::new(moved_data, path))
    }

    /// Map the existing vector into a new one at `Path`, with indexes.
    pub fn map_with_index<O, F>(&self, f: F, path: PathBuf) -> io::Result<MmVec<O>>
    where
        F: Fn(usize, T) -> O,
        O: Copy,
    {
        let mut target = MmVec::with_length_uninit(self.len(), path)?;
        // SAFETY: this is safe because both vectors are valid
        unsafe {
            let src_slice = self.as_slice();
            let dst_slice = target.as_slice_mut();
            for (idx, el) in src_slice.iter().enumerate() {
                dst_slice[idx] = f(idx, *el);
            }
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

    ///  Merge self and other into a single vector, in-place. Both vectors should be sorted!
    pub fn merge_sorted<O, F>(&mut self, other: &[T], sort_key: F) -> io::Result<()>
    where
        F: Fn(T) -> O,
        O: Ord,
    {
        let path = self.path.clone();
        let mut temp_path = path.clone();
        temp_path.set_extension(".merging");
        let merged = self.merge_sorted_into_path(other, sort_key, temp_path)?;
        *self = merged.move_to(path)?;
        Ok(())
    }

    /// Merge self and other into a single vector. Both vectors should be sorted!
    pub fn merge_sorted_into_path<O, F>(&self, other: &[T], sort_key: F, path: PathBuf) -> io::Result<Self>
    where
        F: Fn(T) -> O,
        O: Ord,
    {
        let new_size = self.len() + other.len();
        let mut merged = Self::with_length_uninit(new_size, path)?;
        // SAFETY: this is safe because both vectors are valid
        unsafe { merge_sorted(self.as_slice(), other, merged.as_slice_mut(), sort_key) };

        Ok(merged)
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

/// Low-level memory-mapped data
struct Data<T> {
    header_mmap: MmapMut,
    data_mmap: MmapMut,
    dummy: PhantomData<T>,
}

impl<T> Data<T> {
    const HEADER_SIZE: u64 = 16;

    unsafe fn header_offset(&self, offset: usize) -> *const u8 {
        assert!(offset < Self::HEADER_SIZE as usize, "offset is out of bounds");
        (self.header_mmap.as_ptr() as *const u8).add(offset)
    }

    pub unsafe fn sig(&self) -> u32 {
        *(self.header_offset(0) as *const u32)
    }

    unsafe fn set_sig(&mut self, sig: u32) {
        *(self.header_offset(0) as *mut u32) = sig;
    }

    #[allow(unused)]
    pub unsafe fn crc(&self) -> u32 {
        *(self.header_offset(4) as *const u32)
    }

    unsafe fn set_crc(&mut self, crc: u32) {
        *(self.header_offset(4) as *mut u32) = crc;
    }

    pub unsafe fn len(&self) -> u64 {
        *(self.header_offset(8) as *const u64)
    }

    unsafe fn set_len(&mut self, len: u64) {
        *(self.header_offset(8) as *mut u64) = len;
    }

    pub fn capacity(&self) -> usize {
        self.data_mmap.len() / std::mem::size_of::<T>()
    }

    pub unsafe fn as_ptr(&self) -> *const T {
        self.data_mmap.as_ptr() as *const T
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut T {
        self.data_mmap.as_mut_ptr() as *mut T
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        slice::from_raw_parts(self.as_ptr(), self.len() as usize)
    }

    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        slice::from_raw_parts_mut(self.as_mut_ptr(), self.len() as usize)
    }

    pub unsafe fn from_file_unchecked(file: File) -> io::Result<Self> {
        let len_bytes = file.metadata()?.len();
        let header_mmap = mmap(&file, 0, Self::HEADER_SIZE as usize)?;
        let data_mmap = mmap(&file, Self::HEADER_SIZE, (len_bytes - Self::HEADER_SIZE) as usize)?;
        Ok(Self {
            header_mmap,
            data_mmap,
            dummy: PhantomData,
        })
    }

    pub unsafe fn new_uninit(file: File, len: usize) -> io::Result<Self> {
        let needed_bytes = len * std::mem::size_of::<T>();
        file.set_len(Self::HEADER_SIZE + needed_bytes as u64)?;
        let mut data = Self::from_file_unchecked(file)?;
        data.set_sig(0);
        data.set_crc(0);
        data.set_len(len as u64);
        data.header_mmap.flush()?;
        Ok(data)
    }

    pub unsafe fn new_with_data(mut file: File, data: &[T]) -> io::Result<Self> {
        let len = data.len();
        let bytes = slice_as_bytes(data);
        let crc = crc32fast::hash(bytes);
        file.set_len(Self::HEADER_SIZE + bytes.len() as u64)?;
        file.seek(io::SeekFrom::Start(Self::HEADER_SIZE))?;
        file.write_all(bytes)?;
        file.flush()?;
        let mut data = Self::from_file_unchecked(file)?;
        data.set_sig(0);
        data.set_crc(crc);
        data.set_len(len as u64);
        data.header_mmap.flush()?;
        Ok(data)
    }

    pub fn flush(&self) -> io::Result<()> {
        self.header_mmap.flush()?;
        self.data_mmap.flush()?;
        Ok(())
    }
}

impl<T> Drop for Data<T> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

fn create_new_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(path)
}

fn open_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(false)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
}

fn slice_as_bytes<T>(data: &[T]) -> &[u8] {
    let len_bytes = std::mem::size_of_val(data);
    // SAFETY: this is a valid slice.
    unsafe { slice::from_raw_parts(data.as_ptr() as *const u8, len_bytes) }
}

unsafe fn mmap(file: &File, offset: u64, len: usize) -> io::Result<MmapMut> {
    let mut opts = MmapOptions::new();
    let mmap = opts.offset(offset).len(len).map_mut(file)?;
    #[cfg(unix)]
    {
        mmap.advise(memmap2::Advice::Random).ok();
    }
    Ok(mmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_header_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let test_path = tmp.path().join("test.mmvec");
        unsafe {
            let file = create_new_file(&test_path).unwrap();
            let mut data = Data::<u64>::new_uninit(file, 100).expect("failed to create data");
            assert_eq!(data.sig(), 0, "uninit sig should be 0!");
            assert_eq!(data.crc(), 0, "uninit crc should be 0!");
            assert_eq!(data.len(), 100, "len should be 100!");
            data.set_sig(129);
            assert_eq!(data.sig(), 129, "data sig cannot be set correctly!");
            data.set_crc(345);
            assert_eq!(data.crc(), 345, "data crc cannot be set correctly!");

            drop(data);

            let file = open_file(&test_path).unwrap();
            let data = Data::<u64>::from_file_unchecked(file).expect("failed to create data");
            assert_eq!(data.sig(), 129, "sig is not preserved!");
            assert_eq!(data.crc(), 345, "crc is not preserved!");
            assert_eq!(data.len(), 100, "len is not preserved!");
        }
    }

    #[test]
    fn test_from_vec_to_vec() {
        let tmp = tempfile::tempdir().unwrap();
        let test_path = tmp.path().join("test.mmvec");
        let data = vec![(199, 200), (200, 532), (449, 400)];
        let vec = MmVec::from_vec(data.clone(), test_path.clone()).expect("failed to create memvec");
        drop(vec);
        let result = MmVec::<(i32, i32)>::from_path(test_path).expect("failed to load memvec from file");
        assert_eq!(unsafe { result.as_slice() }, data.as_slice(), "data was corrupted");
    }

    #[test]
    fn test_with_length_uninitialized_works_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let test_path = tmp.path().join("test.mmvec");
        let vec = MmVec::<(i32, i32)>::with_length_uninit(100, test_path.clone()).expect("failed to create memvec");
        drop(vec);
        let result = MmVec::<(i32, i32)>::from_path(test_path).expect("failed to load memvec from file");
        assert_eq!(result.len(), 100, "data was corrupted");
    }
}
