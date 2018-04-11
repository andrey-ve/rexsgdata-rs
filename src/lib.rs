//! Scatter Gather Data Wrapper.

#![cfg_attr(all(feature = "cargo-clippy", feature = "pedantic"), warn(clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", warn(use_self))]
#![deny(warnings, missing_debug_implementations)]
#![doc(html_root_url = "https://docs.rs/sgdata/0.1.8")]

extern crate libc;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::fmt;
use std::iter::{self, FromIterator};
use std::slice;
use std::vec;

use libc::{c_int, c_void, iovec, size_t};
use serde::de::{self, Error};
use serde::ser::{SerializeSeq, SerializeTupleVariant};
use serde::{Serialize, Serializer};

/// High Level wrapper for multiple data representation methods.
#[derive(Debug, Deserialize, PartialEq)]
pub enum SgData {
    /// Classic Scatter Gather list as it comes from C (array of `iovec` elements)
    SgList(SgList),
    /// Vec<u8> scatter-gather list
    SgVec(Vec<Vec<u8>>),
    /// Plain Vec<u8> buffer
    Direct(Vec<u8>),
    /// Special case for `iovec` array that is itself a Rust' `Vec`
    Element(Vec<Element>),
}

impl From<SgList> for SgData {
    fn from(sglist: SgList) -> Self {
        SgData::SgList(sglist)
    }
}

impl From<Vec<u8>> for SgData {
    fn from(vec: Vec<u8>) -> Self {
        SgData::Direct(vec)
    }
}

impl From<Vec<Vec<u8>>> for SgData {
    fn from(sgvec: Vec<Vec<u8>>) -> Self {
        SgData::SgVec(sgvec)
    }
}

impl FromIterator<u8> for SgData {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = u8>,
    {
        let vec = iter.into_iter().collect::<Vec<_>>();
        SgData::Direct(vec)
    }
}

impl FromIterator<Vec<u8>> for SgData {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Vec<u8>>,
    {
        let vec = iter.into_iter().collect::<Vec<_>>();
        SgData::SgVec(vec)
    }
}

impl FromIterator<Element> for SgData {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Element>,
    {
        let vec = iter.into_iter().collect::<Vec<_>>();
        SgData::Element(vec)
    }
}

impl Serialize for SgData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            SgData::SgList(ref sglist) => {
                // While serializing `SgList` variant mimics `SgVec`.
                let mut data = serializer.serialize_tuple_variant("SgData", 1, "SgVec", 1)?;
                data.serialize_field(sglist)?;
                data.end()
            }
            SgData::SgVec(ref sgvec) => {
                let mut data = serializer.serialize_tuple_variant("SgData", 1, "SgVec", 1)?;
                data.serialize_field(sgvec)?;
                data.end()
            }
            SgData::Direct(ref buf) => {
                let mut data = serializer.serialize_tuple_variant("SgData", 2, "Direct", 1)?;
                data.serialize_field(buf)?;
                data.end()
            }
            SgData::Element(ref vec) => {
                // While serializing `Element` variant mimics `SgVec`
                let mut data = serializer.serialize_tuple_variant("SgData", 1, "SgVec", 1)?;
                data.serialize_field(vec)?;
                data.end()
            }
        }
    }
}

impl IntoIterator for SgData {
    type Item = Vec<u8>;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let vec = match self {
            SgData::SgList(_) => unimplemented!(),
            SgData::SgVec(sgvec) => sgvec,
            SgData::Direct(buf) => vec![buf],
            SgData::Element(_) => unimplemented!(),
        };

        vec.into_iter()
    }
}

/// Wrapper for a C-style scatter gather list
#[derive(Debug, PartialEq)]
pub struct SgList {
    /// Pointer to the `iovec` array
    iovec: *const iovec,
    /// Number of `iovec` elements in the array
    count: c_int,
}

impl SgList {
    /// Constructs new `SgList` object from raw arguments
    pub fn new(iovec: *const iovec, count: c_int) -> Self {
        Self { iovec, count }
    }
}

unsafe impl Send for SgList {}
unsafe impl Sync for SgList {}

impl Serialize for SgList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let count = self.count as usize;
        let mut seq = serializer.serialize_seq(Some(count))?;
        for idx in 0..self.count as isize {
            let buf = unsafe {
                let iov = self.iovec.offset(idx);
                let base = (*iov).iov_base as *const u8;
                let len = (*iov).iov_len as usize;
                slice::from_raw_parts(base, len)
            };
            seq.serialize_element(buf)?;
        }
        seq.end()
    }
}

impl<'de> de::Deserialize<'de> for SgList {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        Err(D::Error::custom("Cannot deserialize SgList"))
    }
}

/// Intermediate element that can represent either regular `iovec` or a sequence of zeroes
pub enum Element {
    /// A block of zeroes of a specified size (aka Zero Length Encoding)
    Zle(usize),
    /// Regular `iovec`
    Iovec(iovec),
}

impl Element {
    /// Constructs `Element::Zle` variant of a specified size
    pub fn zero(size: usize) -> Self {
        Element::Zle(size)
    }
}

impl From<iovec> for Element {
    fn from(iovec: iovec) -> Self {
        Element::Iovec(iovec)
    }
}

impl From<(*mut c_void, size_t)> for Element {
    fn from((iov_base, iov_len): (*mut c_void, size_t)) -> Self {
        Element::Iovec(iovec { iov_base, iov_len })
    }
}

impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Element::*;
        match *self {
            Zle(ref size) => write!(f, "Element::Zle({:?})", size),
            Iovec(ref iov) => write!(f, "Element::Iovec({:?}, {:?})", iov.iov_base, iov.iov_len),
        }
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        use Element::*;

        match (self, other) {
            (&Zle(ref size1), &Zle(ref size2)) => size1 == size2,
            (&Iovec(ref iov1), &Iovec(ref iov2)) => {
                iov1.iov_base == iov2.iov_base && iov1.iov_len == iov2.iov_len
            }
            _ => false,
        }
    }
}

unsafe impl Send for Element {}
unsafe impl Sync for Element {}

impl Serialize for Element {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Element::Zle(ref size) => serializer.collect_seq(iter::repeat(0_u8).take(*size)),
            Element::Iovec(ref iov) => {
                let buf = unsafe {
                    let base = (*iov).iov_base as *const u8;
                    let len = (*iov).iov_len as usize;
                    slice::from_raw_parts(base, len)
                };
                serializer.collect_seq(buf)
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for Element {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        Err(D::Error::custom("Cannot deserialize Element"))
    }
}

fn _assert_impls() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    fn assert_clone<T: Clone>() {}

    assert_send::<SgData>();
    assert_sync::<SgData>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_zero() {
        assert_eq!(Element::zero(1024), Element::zero(1024));
    }

    #[test]
    fn element_iovec() {
        let mut buf = [0x55; 1024];
        let iov = buf.as_mut_ptr() as *mut c_void;
        let len = buf.len();
        let iovec = iovec {
            iov_base: iov,
            iov_len: len,
        };
        let e1 = Element::from((iov, len));
        let e2 = Element::from(iovec);
        assert_eq!(e1, e2);
        assert_ne!(e1, Element::zero(1024));
    }
}
