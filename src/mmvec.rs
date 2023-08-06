use core::slice;
use std::{
    fs::{copy, remove_file, rename, File, OpenOptions},
    io,
    marker::PhantomData,
    mem::size_of,
    path::{Path, PathBuf},
};

use memmap2::{MmapMut, MmapOptions};
use thiserror::Error;

use crate::util::partition;

#[derive(Debug, Error)]
pub enum MmVecError {
    #[error("signature does not match: expected: {expected}, got: {actual} ")]
    SignatureMismatch { expected: u64, actual: u64 },
    #[error("loading vectors which are not fully initialized or have trailing data in the file is not supported!")]
    UninitializedVectorLoad {},
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct MmVec<T>
where
    T: Copy,
{
    data: Option<Data<T>>,
    path: PathBuf,
}

impl<T> MmVec<T>
where
    T: Copy,
{
    fn new(data: Data<T>, path: PathBuf) -> Self {
        Self { data: Some(data), path }
    }

    /// Creates an uninitialized vector with given length.
    unsafe fn with_length_uninit(sig: u64, len: usize, path: PathBuf) -> Result<Self, MmVecError> {
        let file = create_new_file(&path)?;
        let data = Data::new_uninit(file, sig, len)?;
        Ok(Self::new(data, path))
    }

    /// Creates a new, empty vector.
    ///
    /// ## Safety
    /// Creating mmap is unsafe. We cannot guarantee that the file has not been tampered with between its creation
    /// and mmapping.
    pub unsafe fn new_empty(sig: u64, path: PathBuf) -> Result<Self, MmVecError> {
        Self::with_length_uninit(sig, 0, path)
    }

    /// Dumps a slice into path, then mmaps it.
    ///
    /// ## Safety
    /// Creating mmap is unsafe.
    pub unsafe fn from_slice(sig: u64, slice: &[T], path: PathBuf) -> Result<Self, MmVecError> {
        let file = create_new_file(&path)?;
        let data = Data::new_with_data(file, sig, slice)?;
        Ok(Self::new(data, path))
    }

    /// Try to create a vector from the given path. Returns error if the signature does not match, or if
    /// the vector is not completely initialized.
    ///
    /// ## Safety
    /// While we do check signature and length, this is still unsafe since we can't guarantee
    /// that the mmapped file has not been tampered with.
    pub unsafe fn from_path(sig: u64, path: PathBuf) -> Result<Self, MmVecError> {
        let file = open_file(&path)?;
        let data = Data::<T>::from_file_unchecked(file)?;
        if data.sig() != sig {
            return Err(MmVecError::SignatureMismatch {
                expected: sig,
                actual: data.sig(),
            });
        }
        // only whole-file, fully initialized vectors are supported
        if data.len() != data.capacity() as u64 {
            return Err(MmVecError::UninitializedVectorLoad {});
        }
        Ok(Self::new(data, path))
    }

    /// Path to the backing file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Length of this vector.
    ///
    /// ## Safety
    /// Unsafe since we can't guarantee that the mmapped file has not been tampered with
    #[must_use]
    pub unsafe fn len(&self) -> usize {
        self.data.as_ref().map_or(0, |d| d.len() as usize)
    }

    /// Signature of this vector.
    ///
    /// ## Safety
    /// Unsafe since we can't guarantee that the mmapped file has not been tampered with
    #[must_use]
    pub unsafe fn sig(&self) -> u64 {
        self.data.as_ref().map_or(u64::MAX, |d| d.sig())
    }

    /// Whether this vector is empty.
    ///
    /// ## Safety
    /// Unsafe since we can't guarantee that the mmapped file has not been tampered with
    #[must_use]
    pub unsafe fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get contents as a slice.
    ///
    /// ## Safety
    /// Unsafe since we can't guarantee that the mmapped file has not been tampered with.
    #[must_use]
    pub unsafe fn as_slice(&self) -> &[T] {
        self.data.as_ref().map_or(&[], |d| unsafe { d.as_slice() })
    }

    /// Get contents as a mutable slice.
    ///
    /// ## Safety
    /// Unsafe since we can't guarantee that the mmapped file has not been tampered with.
    #[must_use]
    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        self.data.as_mut().map_or(&mut [], |d| unsafe { d.as_slice_mut() })
    }

    /// Flushes memory-mapped data into file.
    pub fn flush(&self) -> Result<(), MmVecError> {
        Ok(self.data.as_ref().map_or(Ok(()), |d| d.flush())?)
    }

    /// Destroys self, removing the underlying file.
    pub fn destroy(mut self) -> Result<(), MmVecError> {
        let path = self.path.clone();
        drop(self.data.take());
        remove_file(path)?;
        Ok(())
    }

    /// Copies self into path, and returns a new vector at this path.
    ///
    /// ## Safety
    /// Unsafe since we can't guarantee that the mmapped file has not been tampered with.
    pub unsafe fn copy_to(&self, path: PathBuf) -> Result<Self, MmVecError> {
        self.flush()?;
        copy(&self.path, &path)?;

        let copied = open_file(&path)?;
        let copied_data = Data::from_file_unchecked(copied)?;
        Ok(Self::new(copied_data, path))
    }

    /// Moves self into path, and returns a new vector at this path.
    ///
    /// ## Safety
    /// We can't guarantee that the mmapped file has not been tampered with.
    pub unsafe fn move_to(self, path: PathBuf) -> Result<Self, MmVecError> {
        self.flush()?;
        rename(&self.path, &path)?;
        drop(self);

        let moved = open_file(&path)?;
        let moved_data = Data::from_file_unchecked(moved)?;
        Ok(Self::new(moved_data, path))
    }

    /// Insert items into vector, preserving sorted order.
    /// If the vector was not previously sorted, it will be.
    ///
    /// Input sequence can be sorted to ensure better performance, but it is not required.
    ///
    /// ## Safety
    /// We can't guarantee that the mmapped file has not been tampered with.
    pub unsafe fn insert_sorted<O, F>(&mut self, items: &[T], sort_key: F) -> Result<(), MmVecError>
    where
        F: Fn(&T) -> O,
        O: Ord,
    {
        self.flush()?;
        let current_len = self.len();
        self.resize(current_len + items.len())?;
        self.as_slice_mut()[current_len..].copy_from_slice(items);
        self.as_slice_mut().sort_unstable_by_key(sort_key);
        Ok(())
    }

    /// Remove all items matching the predicate, while preserving the sorted order.
    /// If the vector was not previously sorted, it will be.
    ///
    /// ## Safety
    /// We can't guarantee that the mmapped file has not been tampered with.
    pub unsafe fn remove_matching<O, F, S>(&mut self, predicate: F, sort_key: S) -> Result<(), MmVecError>
    where
        F: Fn(&T) -> bool,
        S: Fn(&T) -> O,
        O: Ord,
    {
        let split = partition(self.as_slice_mut(), |el| !predicate(el));
        self.resize(split)?;
        self.as_slice_mut().sort_unstable_by_key(sort_key);
        Ok(())
    }

    unsafe fn resize(&mut self, new_len: usize) -> Result<(), MmVecError> {
        self.flush()?;

        // On Windows it is required that file is not mapped before resizing.
        // The safest option is to just drop and recreate the Data.
        #[cfg(windows)]
        {
            drop(self.data.take());

            let file = open_file(self.path())?;
            self.data = Some(Data::from_file_unchecked_resized(file, new_len)?);
        }

        #[cfg(not(windows))]
        self.data.as_mut().map_or(Ok(()), |d| d.resize(new_len))?;

        Ok(())
    }
}

/// Low-level memory-mapped data
struct Data<T>
where
    T: Copy,
{
    #[allow(unused)]
    file: File,
    header_mmap: MmapMut,
    data_mmap: MmapMut,
    dummy: PhantomData<T>,
}

impl<T> Data<T>
where
    T: Copy,
{
    const HEADER_SIZE: u64 = 16;

    fn header_offset(&self, offset: usize) -> *const u8 {
        let start = self.header_mmap.as_ptr() as *const u8;
        assert!(offset < Self::HEADER_SIZE as usize, "offset is out of bounds");
        assert!(offset % 8 == 0, "offset is not placed on u64 boundary");
        // SAFETY: we checked prerequisites for `add`
        unsafe { start.add(offset) }
    }

    fn header_offset_mut(&mut self, offset: usize) -> *mut u8 {
        let start = self.header_mmap.as_mut_ptr() as *mut u8;
        assert!(offset < Self::HEADER_SIZE as usize, "offset is out of bounds");
        assert!(offset % 8 == 0, "offset is not placed on u64 boundary");
        // SAFETY: we checked prerequisites for `add`
        unsafe { start.add(offset) }
    }

    pub unsafe fn sig(&self) -> u64 {
        *(self.header_offset(0) as *const u64)
    }

    unsafe fn set_sig(&mut self, sig: u64) {
        *(self.header_offset_mut(0) as *mut u64) = sig;
    }

    pub unsafe fn len(&self) -> u64 {
        *(self.header_offset(8) as *const u64)
    }

    unsafe fn set_len(&mut self, len: u64) {
        *(self.header_offset_mut(8) as *mut u64) = len;
    }

    pub fn capacity(&self) -> usize {
        self.data_mmap.len() / std::mem::size_of::<T>()
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        slice::from_raw_parts(self.data_mmap.as_ptr() as *const T, self.len() as usize)
    }

    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        slice::from_raw_parts_mut(self.data_mmap.as_mut_ptr() as *mut T, self.len() as usize)
    }

    pub unsafe fn from_file_unchecked(file: File) -> io::Result<Self> {
        let len_bytes = file.metadata()?.len();
        let header_mmap = mmap(&file, 0, Self::HEADER_SIZE as usize)?;
        let data_mmap = mmap(&file, Self::HEADER_SIZE, (len_bytes - Self::HEADER_SIZE) as usize)?;
        Ok(Self {
            file,
            header_mmap,
            data_mmap,
            dummy: PhantomData,
        })
    }

    #[allow(unused)]
    pub unsafe fn from_file_unchecked_resized(file: File, len: usize) -> io::Result<Self> {
        resize_file_to_fit::<T>(&file, Self::HEADER_SIZE, len)?;
        let mut data = Self::from_file_unchecked(file)?;
        data.set_len(len as u64);
        Ok(data)
    }

    pub unsafe fn new_uninit(file: File, sig: u64, len: usize) -> io::Result<Self> {
        resize_file_to_fit::<T>(&file, Self::HEADER_SIZE, len)?;
        let mut data = Self::from_file_unchecked(file)?;
        data.set_sig(sig);
        data.set_len(len as u64);
        data.header_mmap.flush()?;
        Ok(data)
    }

    pub unsafe fn new_with_data(file: File, sig: u64, slice: &[T]) -> io::Result<Self> {
        resize_file_to_fit::<T>(&file, Self::HEADER_SIZE, slice.len())?;
        let mut data = Self::from_file_unchecked(file)?;
        data.set_sig(sig);
        data.set_len(slice.len() as u64);
        data.as_slice_mut().copy_from_slice(slice);
        data.header_mmap.flush()?;
        Ok(data)
    }

    #[cfg(not(windows))]
    pub unsafe fn resize(&mut self, len: usize) -> io::Result<()> {
        self.flush()?;
        let new_len_bytes = resize_file_to_fit::<T>(&self.file, Self::HEADER_SIZE, len)?;
        self.data_mmap = mmap(&self.file, Self::HEADER_SIZE, new_len_bytes as usize)?;
        self.set_len(len as u64);
        Ok(())
    }

    pub fn flush(&self) -> io::Result<()> {
        self.header_mmap.flush()?;
        self.data_mmap.flush()?;
        Ok(())
    }
}

impl<T> Drop for Data<T>
where
    T: Copy,
{
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

fn resize_file_to_fit<T>(file: &File, header_size: u64, len: usize) -> io::Result<u64> {
    let needed_bytes = size_of::<T>() as u64 * len as u64;
    file.set_len(header_size + needed_bytes)?;
    Ok(needed_bytes)
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

    fn with_file_path(f: impl FnOnce(&Path)) {
        let tmp = tempfile::tempdir().expect("failed to create tmp dir");
        let test_path = tmp.path().join("test.bin");
        f(&test_path)
    }

    fn get_file_len(path: &Path) -> u64 {
        let file3 = open_file(&path).expect("failed to open file");
        file3.metadata().expect("failed to read metadata").len()
    }

    #[test]
    fn data_header_fields_are_correctly_initialized() {
        with_file_path(|test_path| {
            let file = create_new_file(&test_path).expect("failed to create file");
            unsafe {
                let data = Data::<u64>::new_uninit(file, 42, 100).expect("failed to create data");
                assert_eq!(data.sig(), 42, "sig");
                assert_eq!(data.len(), 100, "len");
            }
        })
    }

    #[test]
    fn data_header_fields_can_be_set() {
        with_file_path(|test_path| {
            let file = create_new_file(&test_path).expect("failed to create file");
            unsafe {
                let mut data = Data::<u64>::new_uninit(file, 42, 100).expect("failed to create data");
                data.set_sig(420);
                data.set_len(1000);
                assert_eq!(data.sig(), 420, "sig");
                assert_eq!(data.len(), 1000, "len");
            }
        })
    }

    #[test]
    fn data_header_fields_can_be_read_after_data_is_recreated() {
        with_file_path(|test_path| {
            let file = create_new_file(&test_path).expect("failed to create file");
            unsafe {
                let mut data = Data::<u64>::new_uninit(file, 42, 100).expect("failed to create data");
                data.set_sig(420);
                data.set_len(1000);
            }

            let file = open_file(&test_path).unwrap();
            unsafe {
                let data = Data::<u64>::from_file_unchecked(file).expect("failed to create data");
                assert_eq!(data.sig(), 420, "sig");
                assert_eq!(data.len(), 1000, "len");
            }
        })
    }

    #[cfg(not(windows))]
    #[test]
    fn data_can_be_correctly_resized_grow() {
        with_file_path(|path| {
            let file = create_new_file(path).expect("failed to create file");
            unsafe {
                let mut data = Data::<u64>::new_uninit(file, 42, 100).expect("failed to create data");
                data.resize(1000).expect("failed to resize data");
                assert_eq!(data.len(), 1000, "updated len");
                assert_eq!(
                    data.data_mmap.len(),
                    1000 * size_of::<u64>(),
                    "mmap size should be able to fit resized data"
                );
                assert_eq!(
                    get_file_len(path),
                    Data::<u64>::HEADER_SIZE + 1000 * size_of::<u64>() as u64,
                    "file should be able to fit resized data"
                );
            }
            assert_eq!(
                get_file_len(path),
                Data::<u64>::HEADER_SIZE + 1000 * size_of::<u64>() as u64,
                "file should preserve resized length after data is destroyed"
            );
        })
    }

    #[cfg(not(windows))]
    #[test]
    fn data_can_be_correctly_resized_shrink() {
        with_file_path(|path| {
            let file = create_new_file(path).expect("failed to create file");
            unsafe {
                let mut data = Data::<u64>::new_uninit(file, 42, 100).expect("failed to create data");
                data.resize(10).expect("failed to resize data");
                assert_eq!(data.len(), 10, "updated len");
                assert_eq!(
                    data.data_mmap.len(),
                    10 * size_of::<u64>(),
                    "mmap size should be able to fit resized data"
                );
                assert_eq!(
                    get_file_len(path),
                    Data::<u64>::HEADER_SIZE + 10 * size_of::<u64>() as u64,
                    "file should be able to fit resized data"
                );
            }
            assert_eq!(
                get_file_len(path),
                Data::<u64>::HEADER_SIZE + 10 * size_of::<u64>() as u64,
                "file should preserve resized length after data is destroyed"
            );
        })
    }

    #[test]
    fn mmvec_can_be_dumped_to_file_then_read() {
        with_file_path(|path| unsafe {
            let data = vec![199, 200, 200, 532, 449, 400];
            let vec = MmVec::from_slice(0, &data, path.to_path_buf()).expect("failed to create memvec");
            drop(vec);
            let result = MmVec::<i32>::from_path(0, path.to_path_buf()).expect("failed to load memvec from file");
            assert_eq!(result.as_slice(), data.as_slice());
        })
    }
}
