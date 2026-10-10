use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use rusteal_ffi::{FPropertyHandle, RustealErrorCode, UObjectHandle};

const MAX_ELEM_BUF: usize = 4096;

use crate::error::{RustealError, RustealResult, check_ffi, ffi_infallible};
use crate::ffi_dispatch;
use crate::object_ref::UObjectRef;
use crate::struct_ref::UStructRef;
use crate::traits::{UeClass, UeStruct};

pub unsafe trait ContainerElement: Sized {
    const BUF_SIZE: u32;

    const RAW_COPYABLE: bool = false;

    unsafe fn read_from_buf(buf: *const u8, written: u32) -> Self;

    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32;
}

macro_rules! impl_container_element_primitive {
    ($ty:ty) => {
        unsafe impl ContainerElement for $ty {
            const BUF_SIZE: u32 = std::mem::size_of::<$ty>() as u32;
            const RAW_COPYABLE: bool = true;

            #[inline]
            unsafe fn read_from_buf(buf: *const u8, _written: u32) -> Self {
                unsafe { (buf as *const $ty).read_unaligned() }
            }

            #[inline]
            unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
                unsafe {
                    (buf as *mut $ty).write_unaligned(*self);

                    std::mem::size_of::<$ty>() as u32
                }
            }
        }
    };
}

impl_container_element_primitive!(bool);
impl_container_element_primitive!(i8);
impl_container_element_primitive!(u8);
impl_container_element_primitive!(i16);
impl_container_element_primitive!(u16);
impl_container_element_primitive!(i32);
impl_container_element_primitive!(u32);
impl_container_element_primitive!(i64);
impl_container_element_primitive!(u64);
impl_container_element_primitive!(f32);
impl_container_element_primitive!(f64);

unsafe impl ContainerElement for UObjectHandle {
    const BUF_SIZE: u32 = std::mem::size_of::<UObjectHandle>() as u32;

    #[inline]
    unsafe fn read_from_buf(buf: *const u8, _written: u32) -> Self {
        unsafe { (buf as *const UObjectHandle).read_unaligned() }
    }

    #[inline]
    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe {
            (buf as *mut UObjectHandle).write_unaligned(*self);

            std::mem::size_of::<UObjectHandle>() as u32
        }
    }
}

unsafe impl ContainerElement for rusteal_ffi::FNameHandle {
    const BUF_SIZE: u32 = 8;
    const RAW_COPYABLE: bool = true;

    #[inline]
    unsafe fn read_from_buf(buf: *const u8, _written: u32) -> Self {
        unsafe { (buf as *const rusteal_ffi::FNameHandle).read_unaligned() }
    }

    #[inline]
    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe {
            (buf as *mut rusteal_ffi::FNameHandle).write_unaligned(*self);

            8
        }
    }
}

unsafe impl ContainerElement for crate::fname::FName {
    const BUF_SIZE: u32 = 8;
    const RAW_COPYABLE: bool = true;

    #[inline]
    unsafe fn read_from_buf(buf: *const u8, written: u32) -> Self {
        crate::fname::FName(unsafe { rusteal_ffi::FNameHandle::read_from_buf(buf, written) })
    }

    #[inline]
    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe { self.0.write_to_buf(buf) }
    }
}

unsafe impl<T: UeClass> ContainerElement for UObjectRef<T> {
    const BUF_SIZE: u32 = std::mem::size_of::<UObjectHandle>() as u32;

    #[inline]
    unsafe fn read_from_buf(buf: *const u8, _written: u32) -> Self {
        unsafe {
            let handle = (buf as *const UObjectHandle).read_unaligned();

            UObjectRef::from_raw(handle)
        }
    }

    #[inline]
    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe {
            (buf as *mut UObjectHandle).write_unaligned(self.raw());

            std::mem::size_of::<UObjectHandle>() as u32
        }
    }
}

unsafe impl ContainerElement for String {
    const BUF_SIZE: u32 = 4096;

    unsafe fn read_from_buf(buf: *const u8, written: u32) -> Self {
        unsafe {
            if written < 4 {
                return String::new();
            }

            let len = (buf as *const u32).read_unaligned() as usize;
            let data_len = (written as usize).saturating_sub(4).min(len);
            let slice = std::slice::from_raw_parts(buf.add(4), data_len);

            String::from_utf8_lossy(slice).into_owned()
        }
    }

    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe {
            let bytes = self.as_bytes();
            let len = bytes.len() as u32;
            (buf as *mut u32).write_unaligned(len);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.add(4), bytes.len());

            4 + len
        }
    }
}

#[derive(Clone, Copy)]
pub struct UeArray<T: ContainerElement> {
    owner: UObjectHandle,
    prop: FPropertyHandle,
    _marker: PhantomData<T>,
}

impl<T: ContainerElement> UeArray<T> {
    #[inline]
    pub fn new(owner: UObjectHandle, prop: FPropertyHandle) -> Self {
        UeArray {
            owner,
            prop,
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> RustealResult<usize> {
        let n = unsafe { ffi_dispatch::container_array_len(self.owner, self.prop) };

        if n < 0 {
            return Err(RustealError::ObjectDestroyed);
        }

        Ok(n as usize)
    }

    pub fn is_empty(&self) -> RustealResult<bool> {
        Ok(self.len()? == 0)
    }

    pub fn get(&self, index: usize) -> RustealResult<T> {
        let mut buf = [0u8; MAX_ELEM_BUF];
        let mut written: u32 = 0;

        check_ffi(unsafe {
            ffi_dispatch::container_array_get(
                self.owner,
                self.prop,
                index as i32,
                buf.as_mut_ptr(),
                T::BUF_SIZE,
                &mut written,
            )
        })?;

        Ok(unsafe { T::read_from_buf(buf.as_ptr(), written) })
    }

    pub fn set(&self, index: usize, val: &T) -> RustealResult<()> {
        let mut buf = [0u8; MAX_ELEM_BUF];

        let written = unsafe { val.write_to_buf(buf.as_mut_ptr()) };

        check_ffi(unsafe {
            ffi_dispatch::container_array_set(
                self.owner,
                self.prop,
                index as i32,
                buf.as_ptr(),
                written,
            )
        })
    }

    pub fn push(&self, val: &T) -> RustealResult<()> {
        let mut buf = [0u8; MAX_ELEM_BUF];

        let written = unsafe { val.write_to_buf(buf.as_mut_ptr()) };

        check_ffi(unsafe {
            ffi_dispatch::container_array_add(self.owner, self.prop, buf.as_ptr(), written)
        })
    }

    pub fn remove(&self, index: usize) -> RustealResult<()> {
        check_ffi(unsafe {
            ffi_dispatch::container_array_remove(self.owner, self.prop, index as i32)
        })
    }

    pub fn clear(&self) -> RustealResult<()> {
        check_ffi(unsafe { ffi_dispatch::container_array_clear(self.owner, self.prop) })
    }

    pub fn iter(&self) -> UeArrayIter<'_, T> {
        let len = self.len().unwrap_or(0);

        UeArrayIter {
            array: self,
            index: 0,
            len,
        }
    }
}

impl<'a, T: ContainerElement> IntoIterator for &'a UeArray<T> {
    type Item = RustealResult<T>;
    type IntoIter = UeArrayIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct UeArrayIter<'a, T: ContainerElement> {
    array: &'a UeArray<T>,
    index: usize,
    len: usize,
}

impl<T: ContainerElement> Iterator for UeArrayIter<'_, T> {
    type Item = RustealResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }

        let result = self.array.get(self.index);
        self.index += 1;

        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.index);

        (remaining, Some(remaining))
    }
}

impl<T: ContainerElement> ExactSizeIterator for UeArrayIter<'_, T> {}

fn bulk_copy_with_retry(
    estimate: usize,
    call: impl Fn(*mut u8, u32, *mut u32, *mut i32) -> RustealErrorCode,
) -> RustealResult<(Vec<u8>, i32)> {
    let mut buf = vec![0u8; estimate.max(64)];
    let mut written: u32 = 0;
    let mut count: i32 = 0;
    let code = call(buf.as_mut_ptr(), buf.len() as u32, &mut written, &mut count);

    if code == RustealErrorCode::BufferTooSmall {
        let needed = (written as usize).max(buf.len() * 2);
        buf.resize(needed, 0);

        check_ffi(call(
            buf.as_mut_ptr(),
            buf.len() as u32,
            &mut written,
            &mut count,
        ))?;
    } else {
        check_ffi(code)?;
    }

    buf.truncate(written as usize);
    Ok((buf, count))
}

pub struct BulkArrayIter<T: ContainerElement> {
    buf: Vec<u8>,
    count: usize,
    index: usize,
    offset: usize,
    raw_elem_size: usize,
    _marker: PhantomData<T>,
}

impl<T: ContainerElement> Iterator for BulkArrayIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.index >= self.count {
            return None;
        }

        if self.raw_elem_size > 0 {
            let elem = unsafe {
                T::read_from_buf(
                    self.buf.as_ptr().add(self.offset),
                    self.raw_elem_size as u32,
                )
            };

            self.offset += self.raw_elem_size;
            self.index += 1;

            Some(elem)
        } else {
            if self.offset + 4 > self.buf.len() {
                return None;
            }

            let written =
                u32::from_ne_bytes(self.buf[self.offset..self.offset + 4].try_into().unwrap());

            self.offset += 4;

            let elem = unsafe { T::read_from_buf(self.buf.as_ptr().add(self.offset), written) };

            self.offset += written as usize;
            self.index += 1;

            Some(elem)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let r = self.count - self.index;

        (r, Some(r))
    }
}

impl<T: ContainerElement> ExactSizeIterator for BulkArrayIter<T> {}

pub struct BulkMapIter<K: ContainerElement, V: ContainerElement> {
    buf: Vec<u8>,
    count: usize,
    index: usize,
    offset: usize,
    _marker: PhantomData<(K, V)>,
}

impl<K: ContainerElement, V: ContainerElement> Iterator for BulkMapIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<(K, V)> {
        if self.index >= self.count {
            return None;
        }

        if self.offset + 4 > self.buf.len() {
            return None;
        }

        let key_written =
            u32::from_ne_bytes(self.buf[self.offset..self.offset + 4].try_into().unwrap());

        self.offset += 4;

        let key = unsafe { K::read_from_buf(self.buf.as_ptr().add(self.offset), key_written) };

        self.offset += key_written as usize;

        if self.offset + 4 > self.buf.len() {
            return None;
        }

        let val_written =
            u32::from_ne_bytes(self.buf[self.offset..self.offset + 4].try_into().unwrap());

        self.offset += 4;

        let val = unsafe { V::read_from_buf(self.buf.as_ptr().add(self.offset), val_written) };

        self.offset += val_written as usize;

        self.index += 1;

        Some((key, val))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let r = self.count - self.index;

        (r, Some(r))
    }
}

impl<K: ContainerElement, V: ContainerElement> ExactSizeIterator for BulkMapIter<K, V> {}

pub struct BulkSetIter<T: ContainerElement> {
    buf: Vec<u8>,
    count: usize,
    index: usize,
    offset: usize,
    _marker: PhantomData<T>,
}

impl<T: ContainerElement> Iterator for BulkSetIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.index >= self.count {
            return None;
        }

        if self.offset + 4 > self.buf.len() {
            return None;
        }

        let written =
            u32::from_ne_bytes(self.buf[self.offset..self.offset + 4].try_into().unwrap());

        self.offset += 4;
        let elem = unsafe { T::read_from_buf(self.buf.as_ptr().add(self.offset), written) };
        self.offset += written as usize;
        self.index += 1;

        Some(elem)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let r = self.count - self.index;

        (r, Some(r))
    }
}

impl<T: ContainerElement> ExactSizeIterator for BulkSetIter<T> {}

impl<T: ContainerElement> UeArray<T> {
    pub fn to_vec(&self) -> RustealResult<Vec<T>> {
        let len = self.len()?;

        if len == 0 {
            return Ok(Vec::new());
        }

        let owner = self.owner;
        let prop = self.prop;

        let estimate = if T::RAW_COPYABLE {
            len * T::BUF_SIZE as usize
        } else {
            len * (T::BUF_SIZE as usize + 4)
        };

        let (buf, count) = bulk_copy_with_retry(estimate, |out, size, written, cnt| unsafe {
            ffi_dispatch::container_array_copy_all(owner, prop, out, size, written, cnt)
        })?;

        let (actual_count, raw_elem_size) = if count < 0 {
            ((-count) as usize, T::BUF_SIZE as usize)
        } else {
            (count as usize, 0)
        };

        Ok(BulkArrayIter::<T> {
            buf,
            count: actual_count,
            index: 0,
            offset: 0,
            raw_elem_size,
            _marker: PhantomData,
        }
        .collect())
    }

    pub fn bulk_iter(&self) -> RustealResult<BulkArrayIter<T>> {
        let len = self.len()?;

        if len == 0 {
            return Ok(BulkArrayIter {
                buf: Vec::new(),
                count: 0,
                index: 0,
                offset: 0,
                raw_elem_size: 0,
                _marker: PhantomData,
            });
        }

        let owner = self.owner;
        let prop = self.prop;

        let estimate = if T::RAW_COPYABLE {
            len * T::BUF_SIZE as usize
        } else {
            len * (T::BUF_SIZE as usize + 4)
        };

        let (buf, count) = bulk_copy_with_retry(estimate, |out, size, written, cnt| unsafe {
            ffi_dispatch::container_array_copy_all(owner, prop, out, size, written, cnt)
        })?;

        let (actual_count, raw_elem_size) = if count < 0 {
            ((-count) as usize, T::BUF_SIZE as usize)
        } else {
            (count as usize, 0)
        };

        Ok(BulkArrayIter {
            buf,
            count: actual_count,
            index: 0,
            offset: 0,
            raw_elem_size,
            _marker: PhantomData,
        })
    }

    pub fn set_all(&self, items: &[T]) -> RustealResult<()> {
        if items.is_empty() {
            return self.clear();
        }

        if T::RAW_COPYABLE {
            let elem_size = T::BUF_SIZE as usize;
            let mut buf = vec![0u8; items.len() * elem_size];

            for (i, item) in items.iter().enumerate() {
                unsafe {
                    item.write_to_buf(buf.as_mut_ptr().add(i * elem_size));
                }
            }

            check_ffi(unsafe {
                ffi_dispatch::container_array_set_all(
                    self.owner,
                    self.prop,
                    buf.as_ptr(),
                    buf.len() as u32,
                    -(items.len() as i32),
                )
            })
        } else {
            let mut buf = Vec::with_capacity(items.len() * (T::BUF_SIZE as usize + 4));
            let mut elem_buf = [0u8; MAX_ELEM_BUF];

            for item in items {
                let written = unsafe { item.write_to_buf(elem_buf.as_mut_ptr()) };
                buf.extend_from_slice(&written.to_ne_bytes());
                buf.extend_from_slice(&elem_buf[..written as usize]);
            }

            check_ffi(unsafe {
                ffi_dispatch::container_array_set_all(
                    self.owner,
                    self.prop,
                    buf.as_ptr(),
                    buf.len() as u32,
                    items.len() as i32,
                )
            })
        }
    }
}

#[derive(Clone, Copy)]
pub struct UeMap<K: ContainerElement, V: ContainerElement> {
    owner: UObjectHandle,
    prop: FPropertyHandle,
    _marker: PhantomData<(K, V)>,
}

impl<K: ContainerElement, V: ContainerElement> UeMap<K, V> {
    #[inline]
    pub fn new(owner: UObjectHandle, prop: FPropertyHandle) -> Self {
        UeMap {
            owner,
            prop,
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> RustealResult<usize> {
        let n = unsafe { ffi_dispatch::container_map_len(self.owner, self.prop) };

        if n < 0 {
            return Err(RustealError::ObjectDestroyed);
        }

        Ok(n as usize)
    }

    pub fn is_empty(&self) -> RustealResult<bool> {
        Ok(self.len()? == 0)
    }

    pub fn find(&self, key: &K) -> RustealResult<V> {
        let mut key_buf = [0u8; MAX_ELEM_BUF];
        let key_written = unsafe { key.write_to_buf(key_buf.as_mut_ptr()) };
        let mut val_buf = [0u8; MAX_ELEM_BUF];
        let mut val_written: u32 = 0;

        check_ffi(unsafe {
            ffi_dispatch::container_map_find(
                self.owner,
                self.prop,
                key_buf.as_ptr(),
                key_written,
                val_buf.as_mut_ptr(),
                V::BUF_SIZE,
                &mut val_written,
            )
        })?;

        Ok(unsafe { V::read_from_buf(val_buf.as_ptr(), val_written) })
    }

    pub fn add(&self, key: &K, val: &V) -> RustealResult<()> {
        let mut key_buf = [0u8; MAX_ELEM_BUF];
        let key_written = unsafe { key.write_to_buf(key_buf.as_mut_ptr()) };
        let mut val_buf = [0u8; MAX_ELEM_BUF];
        let val_written = unsafe { val.write_to_buf(val_buf.as_mut_ptr()) };

        check_ffi(unsafe {
            ffi_dispatch::container_map_add(
                self.owner,
                self.prop,
                key_buf.as_ptr(),
                key_written,
                val_buf.as_ptr(),
                val_written,
            )
        })
    }

    pub fn remove(&self, key: &K) -> RustealResult<()> {
        let mut key_buf = [0u8; MAX_ELEM_BUF];
        let key_written = unsafe { key.write_to_buf(key_buf.as_mut_ptr()) };

        check_ffi(unsafe {
            ffi_dispatch::container_map_remove(self.owner, self.prop, key_buf.as_ptr(), key_written)
        })
    }

    pub fn clear(&self) -> RustealResult<()> {
        check_ffi(unsafe { ffi_dispatch::container_map_clear(self.owner, self.prop) })
    }

    pub fn get_pair(&self, logical_index: usize) -> RustealResult<(K, V)> {
        let mut key_buf = [0u8; MAX_ELEM_BUF];
        let mut key_written: u32 = 0;
        let mut val_buf = [0u8; MAX_ELEM_BUF];
        let mut val_written: u32 = 0;

        check_ffi(unsafe {
            ffi_dispatch::container_map_get_pair(
                self.owner,
                self.prop,
                logical_index as i32,
                key_buf.as_mut_ptr(),
                K::BUF_SIZE,
                &mut key_written,
                val_buf.as_mut_ptr(),
                V::BUF_SIZE,
                &mut val_written,
            )
        })?;

        Ok(unsafe {
            (
                K::read_from_buf(key_buf.as_ptr(), key_written),
                V::read_from_buf(val_buf.as_ptr(), val_written),
            )
        })
    }

    pub fn iter(&self) -> UeMapIter<'_, K, V> {
        let len = self.len().unwrap_or(0);

        UeMapIter {
            map: self,
            index: 0,
            len,
        }
    }
}

impl<'a, K: ContainerElement, V: ContainerElement> IntoIterator for &'a UeMap<K, V> {
    type Item = RustealResult<(K, V)>;
    type IntoIter = UeMapIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct UeMapIter<'a, K: ContainerElement, V: ContainerElement> {
    map: &'a UeMap<K, V>,
    index: usize,
    len: usize,
}

impl<K: ContainerElement, V: ContainerElement> Iterator for UeMapIter<'_, K, V> {
    type Item = RustealResult<(K, V)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }

        let result = self.map.get_pair(self.index);
        self.index += 1;

        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.index);

        (remaining, Some(remaining))
    }
}

impl<K: ContainerElement, V: ContainerElement> ExactSizeIterator for UeMapIter<'_, K, V> {}

impl<K: ContainerElement, V: ContainerElement> UeMap<K, V> {
    pub fn bulk_iter(&self) -> RustealResult<BulkMapIter<K, V>> {
        let len = self.len()?;

        if len == 0 {
            return Ok(BulkMapIter {
                buf: Vec::new(),
                count: 0,
                index: 0,
                offset: 0,
                _marker: PhantomData,
            });
        }

        let owner = self.owner;
        let prop = self.prop;
        let estimate = len * (K::BUF_SIZE as usize + V::BUF_SIZE as usize + 8);

        let (buf, count) = bulk_copy_with_retry(estimate, |out, size, written, cnt| unsafe {
            ffi_dispatch::container_map_copy_all(owner, prop, out, size, written, cnt)
        })?;

        Ok(BulkMapIter {
            buf,
            count: count as usize,
            index: 0,
            offset: 0,
            _marker: PhantomData,
        })
    }
}

impl<K: ContainerElement + Hash + Eq, V: ContainerElement> UeMap<K, V> {
    pub fn to_hash_map(&self) -> RustealResult<HashMap<K, V>> {
        let iter = self.bulk_iter()?;
        let count = iter.count;
        let mut map = HashMap::with_capacity(count);

        for (k, v) in iter {
            map.insert(k, v);
        }

        Ok(map)
    }
}

#[derive(Clone, Copy)]
pub struct UeSet<T: ContainerElement> {
    owner: UObjectHandle,
    prop: FPropertyHandle,
    _marker: PhantomData<T>,
}

impl<T: ContainerElement> UeSet<T> {
    #[inline]
    pub fn new(owner: UObjectHandle, prop: FPropertyHandle) -> Self {
        UeSet {
            owner,
            prop,
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> RustealResult<usize> {
        let n = unsafe { ffi_dispatch::container_set_len(self.owner, self.prop) };

        if n < 0 {
            return Err(RustealError::ObjectDestroyed);
        }

        Ok(n as usize)
    }

    pub fn is_empty(&self) -> RustealResult<bool> {
        Ok(self.len()? == 0)
    }

    pub fn contains(&self, val: &T) -> RustealResult<bool> {
        let mut buf = [0u8; MAX_ELEM_BUF];
        let written = unsafe { val.write_to_buf(buf.as_mut_ptr()) };

        Ok(unsafe {
            ffi_dispatch::container_set_contains(self.owner, self.prop, buf.as_ptr(), written)
        })
    }

    pub fn add(&self, val: &T) -> RustealResult<()> {
        let mut buf = [0u8; MAX_ELEM_BUF];
        let written = unsafe { val.write_to_buf(buf.as_mut_ptr()) };

        check_ffi(unsafe {
            ffi_dispatch::container_set_add(self.owner, self.prop, buf.as_ptr(), written)
        })
    }

    pub fn remove(&self, val: &T) -> RustealResult<()> {
        let mut buf = [0u8; MAX_ELEM_BUF];
        let written = unsafe { val.write_to_buf(buf.as_mut_ptr()) };

        check_ffi(unsafe {
            ffi_dispatch::container_set_remove(self.owner, self.prop, buf.as_ptr(), written)
        })
    }

    pub fn clear(&self) -> RustealResult<()> {
        check_ffi(unsafe { ffi_dispatch::container_set_clear(self.owner, self.prop) })
    }

    pub fn get_element(&self, logical_index: usize) -> RustealResult<T> {
        let mut buf = [0u8; MAX_ELEM_BUF];
        let mut written: u32 = 0;

        check_ffi(unsafe {
            ffi_dispatch::container_set_get_element(
                self.owner,
                self.prop,
                logical_index as i32,
                buf.as_mut_ptr(),
                T::BUF_SIZE,
                &mut written,
            )
        })?;

        Ok(unsafe { T::read_from_buf(buf.as_ptr(), written) })
    }

    pub fn iter(&self) -> UeSetIter<'_, T> {
        let len = self.len().unwrap_or(0);

        UeSetIter {
            set: self,
            index: 0,
            len,
        }
    }
}

impl<'a, T: ContainerElement> IntoIterator for &'a UeSet<T> {
    type Item = RustealResult<T>;
    type IntoIter = UeSetIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct UeSetIter<'a, T: ContainerElement> {
    set: &'a UeSet<T>,
    index: usize,
    len: usize,
}

impl<T: ContainerElement> Iterator for UeSetIter<'_, T> {
    type Item = RustealResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }

        let result = self.set.get_element(self.index);
        self.index += 1;

        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len.saturating_sub(self.index);

        (remaining, Some(remaining))
    }
}

impl<T: ContainerElement> ExactSizeIterator for UeSetIter<'_, T> {}

impl<T: ContainerElement> UeSet<T> {
    pub fn bulk_iter(&self) -> RustealResult<BulkSetIter<T>> {
        let len = self.len()?;

        if len == 0 {
            return Ok(BulkSetIter {
                buf: Vec::new(),
                count: 0,
                index: 0,
                offset: 0,
                _marker: PhantomData,
            });
        }

        let owner = self.owner;
        let prop = self.prop;
        let estimate = len * (T::BUF_SIZE as usize + 4);

        let (buf, count) = bulk_copy_with_retry(estimate, |out, size, written, cnt| unsafe {
            ffi_dispatch::container_set_copy_all(owner, prop, out, size, written, cnt)
        })?;

        Ok(BulkSetIter {
            buf,
            count: count as usize,
            index: 0,
            offset: 0,
            _marker: PhantomData,
        })
    }
}

impl<T: ContainerElement + Hash + Eq> UeSet<T> {
    pub fn to_hash_set(&self) -> RustealResult<HashSet<T>> {
        let iter = self.bulk_iter()?;
        let count = iter.count;
        let mut set = HashSet::with_capacity(count);

        for elem in iter {
            set.insert(elem);
        }

        Ok(set)
    }
}

pub struct OwnedStruct<T: UeStruct> {
    data: Vec<u8>,
    needs_destroy: bool,
    _marker: PhantomData<T>,
}

impl<T: UeStruct> Default for OwnedStruct<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: UeStruct> OwnedStruct<T> {
    pub fn new() -> Self {
        let ustruct = T::static_struct();
        let size = unsafe { ffi_dispatch::reflection_get_struct_size(ustruct) };

        debug_assert!(
            size > 0,
            "get_struct_size returned 0 for {}",
            std::any::type_name::<T>()
        );

        let mut data = vec![0u8; size as usize];

        ffi_infallible(unsafe {
            ffi_dispatch::reflection_initialize_struct(ustruct, data.as_mut_ptr())
        });

        OwnedStruct {
            data,
            needs_destroy: true,
            _marker: PhantomData,
        }
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        OwnedStruct {
            data,
            needs_destroy: false,
            _marker: PhantomData,
        }
    }

    pub fn copy_from(src: &UStructRef<T>) -> Self {
        let mut owned = Self::new();

        ffi_infallible(unsafe {
            ffi_dispatch::reflection_copy_struct(
                T::static_struct(),
                owned.data.as_mut_ptr(),
                src.as_ptr().0 as *const u8,
            )
        });

        owned
    }

    pub fn as_ref(&self) -> UStructRef<T> {
        unsafe { UStructRef::from_raw(self.data.as_ptr() as *mut u8) }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.clone()
    }
}

impl<T: UeStruct> Clone for OwnedStruct<T> {
    fn clone(&self) -> Self {
        OwnedStruct {
            data: self.data.clone(),
            needs_destroy: false,
            _marker: PhantomData,
        }
    }
}

impl<T: UeStruct> Drop for OwnedStruct<T> {
    fn drop(&mut self) {
        if self.needs_destroy {
            unsafe {
                ffi_dispatch::reflection_destroy_struct(T::static_struct(), self.data.as_mut_ptr());
            }
        }
    }
}

impl<T: UeStruct> std::fmt::Debug for OwnedStruct<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedStruct")
            .field("size", &self.data.len())
            .finish()
    }
}

unsafe impl<T: UeStruct> ContainerElement for OwnedStruct<T> {
    const BUF_SIZE: u32 = 4096;

    unsafe fn read_from_buf(buf: *const u8, written: u32) -> Self {
        unsafe {
            let data = vec![0u8; written as usize];
            std::ptr::copy_nonoverlapping(buf, data.as_ptr() as *mut u8, written as usize);

            OwnedStruct::from_bytes(data)
        }
    }

    unsafe fn write_to_buf(&self, buf: *mut u8) -> u32 {
        unsafe {
            let bytes = self.to_bytes();
            let len = bytes.len();
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, len);

            len as u32
        }
    }
}
