//! Fixture definitions for holochain_types structs

#![allow(missing_docs)]

use crate::dna::zome::HostFnAccess;
use crate::dna::zome::Permission;
use crate::dna::zome::Zome;
use crate::dna::zome::ZomeDef;
use crate::dna::DnaDef;
use ::fixt::prelude::*;
use holo_hash::fixt::*;
use holochain_types::fixt::*;
use holochain_wasm_test_utils::TestWasm;
use rand::seq::IteratorRandom;
use std::iter::Iterator;

#[derive(derive_more::Into)]
pub struct Zomes(pub Vec<TestWasm>);

fixturator!(
    Zomes;
    curve Empty Zomes(Vec::new());
    curve Unpredictable {
        // @todo implement unpredictable zomes
        ZomesFixturator::new(Empty).next().unwrap()
    };
    curve Predictable {
        // @todo implement predictable zomes
        ZomesFixturator::new(Empty).next().unwrap()
    };
);

fixturator!(
    Zome;
    constructor fn new(ZomeName, ZomeDef);
);

fixturator!(
    ZomeDef;
    constructor fn from_hash(WasmHash);
);

fixturator!(
    DnaDef;
    curve Empty DnaDef {
        name: StringFixturator::new_indexed(Empty, get_fixt_index!())
            .next()
            .unwrap(),
        uuid: StringFixturator::new_indexed(Empty, get_fixt_index!())
            .next()
            .unwrap(),
        properties: SerializedBytesFixturator::new_indexed(Empty, get_fixt_index!())
            .next()
            .unwrap(),
        zomes: ZomesFixturator::new_indexed(Empty, get_fixt_index!())
            .next()
            .unwrap()
            .0
            .into_iter()
            .map(|w| Zome::from(w).into_inner())
            .collect(),
    };

    curve Unpredictable DnaDef {
        name: StringFixturator::new_indexed(Unpredictable, get_fixt_index!())
            .next()
            .unwrap(),
        uuid: StringFixturator::new_indexed(Unpredictable, get_fixt_index!())
            .next()
            .unwrap(),
        properties: SerializedBytesFixturator::new_indexed(Unpredictable, get_fixt_index!())
            .next()
            .unwrap(),
        zomes: ZomesFixturator::new_indexed(Unpredictable, get_fixt_index!())
            .next()
            .unwrap()
            .0
            .into_iter()
            .map(|w| Zome::from(w).into_inner())
            .collect(),
    };

    curve Predictable DnaDef {
        name: StringFixturator::new_indexed(Predictable, get_fixt_index!())
            .next()
            .unwrap(),
        uuid: StringFixturator::new_indexed(Predictable, get_fixt_index!())
            .next()
            .unwrap(),
        properties: SerializedBytesFixturator::new_indexed(Predictable, get_fixt_index!())
            .next()
            .unwrap(),
        zomes: ZomesFixturator::new_indexed(Predictable, get_fixt_index!())
            .next()
            .unwrap()
            .0
            .into_iter()
            .map(|w| Zome::from(w).into_inner())
            .collect(),
    };
);

fixturator!(
    Permission;
    unit variants [ Allow Deny ] empty Deny;
);

fixturator!(
    HostFnAccess;
    constructor fn new(Permission, Permission, Permission, Permission, Permission, Permission, Permission);
);
