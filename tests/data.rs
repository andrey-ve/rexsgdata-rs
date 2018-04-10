extern crate bincode;
extern crate libc;
extern crate rexsgdata;
extern crate serde;
extern crate serde_test;

use std::mem;

use bincode::{deserialize, serialize};
use libc::{c_int, c_void, iovec};
use rexsgdata::{SgData, SgList};
use serde_test::{assert_ser_tokens, Token};

fn vec_into_iovec(mut vec: Vec<u8>) -> iovec {
    let len = vec.len();
    let base = vec.as_mut_ptr();
    mem::forget(vec);
    iovec {
        iov_base: base as *mut c_void,
        iov_len: len,
    }
}

fn create_sglist(sgvec: Vec<Vec<u8>>) -> SgList {
    let vec = sgvec.into_iter().map(vec_into_iovec).collect::<Vec<_>>();
    let len = vec.len();
    let iov = vec.as_ptr();
    mem::forget(vec);
    SgList::new(iov, len as c_int)
}

#[test]
fn sglist_serialize_deserialize() {
    let sgvec = vec![vec![0x45_u8; 4096]; 5];
    let data = SgData::from(create_sglist(sgvec));
    let buf = serialize(&data).unwrap();
    let data: SgData = deserialize(&buf).unwrap();

    assert_eq!(data, SgData::from(vec![vec![0x45_u8; 4096]; 5]));
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
