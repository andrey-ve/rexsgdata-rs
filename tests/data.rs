extern crate bincode;
extern crate libc;
extern crate rexsgdata;
extern crate serde;
extern crate serde_test;

use std::mem;

use bincode::{deserialize, serialize};
use libc::{c_int, c_void, iovec};
use rexsgdata::{Element, SgData, SgList};
use serde_test::{assert_ser_tokens, Token};

// NB - never use this code outside of the tests - it leaks memory
fn vec_into_iovec(mut vec: Vec<u8>) -> iovec {
    let len = vec.len();
    let base = vec.as_mut_ptr();
    mem::forget(vec);
    iovec {
        iov_base: base as *mut c_void,
        iov_len: len,
    }
}

// NB - never use this code outside of the tests - it leaks memory
fn create_sglist(sgvec: Vec<Vec<u8>>) -> SgList {
    let vec = sgvec.into_iter().map(vec_into_iovec).collect::<Vec<_>>();
    let len = vec.len();
    let iov = vec.as_ptr();
    mem::forget(vec);
    SgList::new(iov, len as c_int)
}

#[test]
fn sglist_serde() {
    let sgvec = vec![vec![0x45_u8; 4096]; 5];
    let data = SgData::from(create_sglist(sgvec));
    let buf = serialize(&data).unwrap();
    let data: SgData = deserialize(&buf).unwrap();

    assert_eq!(data, SgData::from(vec![vec![0x45_u8; 4096]; 5]));
}

#[test]
fn element_serde() {
    let data: SgData = vec![vec![0x46; 4096]; 7]
        .into_iter()
        .map(vec_into_iovec)
        .map(Element::from)
        .collect();

    let buf = serialize(&data).unwrap();
    let data: SgData = deserialize(&buf).unwrap();

    assert_eq!(data, SgData::from(vec![vec![0x46; 4096]; 7]));
}

#[test]
fn direct() {
    let data: SgData = vec![12, 56, 34, 255, 0].into();

    assert_ser_tokens(
        &data,
        &[
            Token::TupleVariant {
                name: "SgData",
                variant: "Direct",
                len: 1,
            },
            Token::Seq { len: Some(5) },
            Token::U8(12),
            Token::U8(56),
            Token::U8(34),
            Token::U8(255),
            Token::U8(0),
            Token::SeqEnd,
            Token::TupleVariantEnd,
        ],
    );
}

#[test]
fn sgvec() {
    let data: SgData = vec![vec![12, 56, 76], vec![128, 255]].into();

    assert_ser_tokens(
        &data,
        &[
            Token::TupleVariant {
                name: "SgData",
                variant: "SgVec",
                len: 1,
            },
            Token::Seq { len: Some(2) },
            Token::Seq { len: Some(3) },
            Token::U8(12),
            Token::U8(56),
            Token::U8(76),
            Token::SeqEnd,
            Token::Seq { len: Some(2) },
            Token::U8(128),
            Token::U8(255),
            Token::SeqEnd,
            Token::SeqEnd,
            Token::TupleVariantEnd,
        ],
    );
}

#[test]
fn sglist() {
    let data: SgData = create_sglist(vec![vec![12, 56, 76], vec![128, 255]]).into();

    assert_ser_tokens(
        &data,
        &[
            Token::TupleVariant {
                name: "SgData",
                variant: "SgVec",
                len: 1,
            },
            Token::Seq { len: Some(2) },
            Token::Seq { len: Some(3) },
            Token::U8(12),
            Token::U8(56),
            Token::U8(76),
            Token::SeqEnd,
            Token::Seq { len: Some(2) },
            Token::U8(128),
            Token::U8(255),
            Token::SeqEnd,
            Token::SeqEnd,
            Token::TupleVariantEnd,
        ],
    );
}

#[test]
fn element_zero() {
    let data: SgData = vec![Element::zero(4), Element::zero(5)]
        .into_iter()
        .collect();
    assert_ser_tokens(
        &data,
        &[
            Token::TupleVariant {
                name: "SgData",
                variant: "SgVec",
                len: 1,
            },
            Token::Seq { len: Some(2) },
            Token::Seq { len: Some(4) },
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::SeqEnd,
            Token::Seq { len: Some(5) },
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::SeqEnd,
            Token::SeqEnd,
            Token::TupleVariantEnd,
        ],
    );
}

#[test]
fn element_iovec() {
    let data: SgData = vec![vec![36, 123, 234], vec![87, 187, 211, 45]]
        .into_iter()
        .map(vec_into_iovec)
        .map(Element::from)
        .collect();

    assert_ser_tokens(
        &data,
        &[
            Token::TupleVariant {
                name: "SgData",
                variant: "SgVec",
                len: 1,
            },
            Token::Seq { len: Some(2) },
            Token::Seq { len: Some(3) },
            Token::U8(36),
            Token::U8(123),
            Token::U8(234),
            Token::SeqEnd,
            Token::Seq { len: Some(4) },
            Token::U8(87),
            Token::U8(187),
            Token::U8(211),
            Token::U8(45),
            Token::SeqEnd,
            Token::SeqEnd,
            Token::TupleVariantEnd,
        ],
    );
}

#[test]
fn element_mixed() {
    let data: SgData = vec![vec![36, 123, 234]]
        .into_iter()
        .map(vec_into_iovec)
        .map(Element::from)
        .chain(Some(Element::zero(5)))
        .collect();

    assert_ser_tokens(
        &data,
        &[
            Token::TupleVariant {
                name: "SgData",
                variant: "SgVec",
                len: 1,
            },
            Token::Seq { len: Some(2) },
            Token::Seq { len: Some(3) },
            Token::U8(36),
            Token::U8(123),
            Token::U8(234),
            Token::SeqEnd,
            Token::Seq { len: Some(5) },
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::U8(0),
            Token::SeqEnd,
            Token::SeqEnd,
            Token::TupleVariantEnd,
        ],
    );
}
