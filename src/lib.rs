#![cfg_attr(all(feature = "cargo-clippy", feature = "pedantic"), warn(clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", warn(use_self))]
#![deny(warnings, missing_debug_implementations)]

extern crate libc;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use std::slice;
use std::vec;

use libc::{c_int, iovec};
use serde::de::{self, Error};
use serde::ser::{SerializeSeq, SerializeTupleVariant};
use serde::{Serialize, Serializer};

#[derive(Debug, Deserialize, PartialEq)]
pub enum SgData {
    SgList(SgList),
    SgVec(Vec<Vec<u8>>),
    Direct(Vec<u8>),
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
