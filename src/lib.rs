//! Scatter Gather Data Wrapper.

#![cfg_attr(all(feature = "cargo-clippy", feature = "pedantic"), warn(clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", warn(use_self))]
#![deny(warnings, missing_debug_implementations)]
#![doc(html_root_url = "https://docs.rs/sgdata/0.1.5")]

extern crate libc;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::fmt;
use std::iter::FromIterator;
use std::slice;
use std::vec;

use libc::{c_int, c_void, iovec, size_t};
use serde::de::{self, Error};
use serde::ser::{SerializeSeq, SerializeTupleVariant};
use serde::{Serialize, Serializer};

#[derive(Debug, Deserialize, PartialEq)]
pub enum SgData {
    SgList(SgList),
    SgVec(Vec<Vec<u8>>),
    Direct(Vec<u8>),
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
            SgData::Element(ref _vec) => unimplemented!(),
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

#[derive(Debug, PartialEq)]
pub struct SgList {
    iovec: *const iovec,
    count: c_int,
}

impl SgList {
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

pub enum Element {
    Zero(usize),
    Iovec(iovec),
}

impl Element {
    pub fn zero(size: usize) -> Self {
        Element::Zero(size)
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
            Zero(size) => write!(f, "Element::Zero({})", size),
            Iovec(ref iov) => write!(f, "Element::Iovec({:?}, {:?})", iov.iov_base, iov.iov_len),
        }
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        use Element::*;

        if let Zero(ref size) = *self {
            if let Zero(ref other) = *other {
                return size == other;
            }
        }

        if let Iovec(ref iovec) = *self {
            if let Iovec(ref other) = *other {
                return (iovec.iov_base == other.iov_base) && (iovec.iov_len == other.iov_len);
            }
        }

        false
    }
}

unsafe impl Send for Element {}
unsafe impl Sync for Element {}

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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
