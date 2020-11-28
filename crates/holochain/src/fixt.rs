pub mod curve;

use crate::conductor::api::CellConductorApi;
use crate::conductor::api::CellConductorReadHandle;
use crate::conductor::handle::MockConductorHandleT;
use crate::conductor::interface::SignalBroadcaster;
use crate::core::ribosome::guest_callback::entry_defs::EntryDefsHostAccess;
use crate::core::ribosome::guest_callback::entry_defs::EntryDefsInvocation;
use crate::core::ribosome::guest_callback::init::InitHostAccess;
use crate::core::ribosome::guest_callback::init::InitInvocation;
use crate::core::ribosome::guest_callback::migrate_agent::MigrateAgentHostAccess;
use crate::core::ribosome::guest_callback::migrate_agent::MigrateAgentInvocation;
use crate::core::ribosome::guest_callback::post_commit::PostCommitHostAccess;
use crate::core::ribosome::guest_callback::post_commit::PostCommitInvocation;
use crate::core::ribosome::guest_callback::validate::ValidateHostAccess;
use crate::core::ribosome::guest_callback::validate::ValidateInvocation;
use crate::core::ribosome::guest_callback::validate_link::ValidateCreateLinkInvocation;
use crate::core::ribosome::guest_callback::validate_link::ValidateDeleteLinkInvocation;
use crate::core::ribosome::guest_callback::validate_link::ValidateLinkHostAccess;
use crate::core::ribosome::guest_callback::validate_link::ValidateLinkInvocation;
use crate::core::ribosome::guest_callback::validation_package::ValidationPackageHostAccess;
use crate::core::ribosome::guest_callback::validation_package::ValidationPackageInvocation;
use crate::core::ribosome::real_ribosome::RealRibosome;
use crate::core::ribosome::CallContext;
use crate::core::ribosome::FnComponents;
use crate::core::ribosome::HostAccess;
use crate::core::ribosome::ZomeCallHostAccess;
use crate::core::ribosome::ZomesToInvoke;
use crate::core::state::metadata::LinkMetaVal;
use crate::core::workflow::CallZomeWorkspace;
use crate::core::workflow::CallZomeWorkspaceLock;
use ::fixt::prelude::*;
pub use holo_hash::fixt::*;
use holo_hash::EntryHash;
use holo_hash::HeaderHash;
use holo_hash::WasmHash;
use holochain_keystore::keystore_actor::KeystoreSender;
use holochain_lmdb::test_utils::test_keystore;
use holochain_p2p::HolochainP2pCellFixturator;
use holochain_types::cell::CellId;
use holochain_types::dna::wasm::DnaWasm;
use holochain_types::dna::zome::WasmZome;
use holochain_types::dna::zome::Zome;
use holochain_types::dna::DnaDefHashed;
use holochain_types::dna::DnaFile;
use holochain_types::dna::Wasms;
use holochain_types::dna::Zomes;
pub use holochain_types::fixt::*;
use holochain_types::test_utils::fake_dna_zomes;
use holochain_wasm_test_utils::strum::IntoEnumIterator;
use holochain_wasm_test_utils::TestWasm;
use holochain_zome_types::element::Element;
use holochain_zome_types::header::HeaderHashes;
use holochain_zome_types::link::LinkTag;
use holochain_zome_types::ExternInput;
use rand::seq::IteratorRandom;
use rand::thread_rng;
use rand::Rng;
use std::collections::BTreeMap;
use std::sync::Arc;

wasm_io_fixturator!(ExternInput<SerializedBytes>);

newtype_fixturator!(FnComponents<Vec<String>>);

fixturator!(
    RealRibosome;
    constructor fn new(DnaFile);
);

impl Iterator for RealRibosomeFixturator<curve::Zomes> {
    type Item = RealRibosome;

    fn next(&mut self) -> Option<Self::Item> {
        // @todo fixturate this
        let dna_file = fake_dna_zomes(
            &StringFixturator::new(Unpredictable).next().unwrap(),
            self.0
                .curve
                .0
                .clone()
                .into_iter()
                .map(|t| (t.into(), t.into()))
                .collect(),
        );

        let ribosome = RealRibosome::new(dna_file);

        // warm the module cache for each wasm in the ribosome
        for zome in self.0.curve.0.clone() {
            let mut call_context = CallContextFixturator::new(Empty).next().unwrap();
            call_context.zome = zome.into();
            ribosome.module(call_context.zome.zome_name()).unwrap();
        }

        self.0.index += 1;

        Some(ribosome)
    }
}

fixturator!(
    DnaWasm;
    // note that an empty wasm will not compile
    curve Empty DnaWasm { code: Arc::new(vec![]) };
    curve Unpredictable TestWasm::iter().choose(&mut thread_rng()).unwrap().into();
    curve Predictable TestWasm::iter().cycle().nth(get_fixt_index!()).unwrap().into();
);

fixturator!(
    Wasms;
    curve Empty BTreeMap::new();
    curve Unpredictable {
        let mut rng = rand::thread_rng();
        let number_of_wasms = rng.gen_range(0, 5);

        let mut wasms: Wasms = BTreeMap::new();
        let mut dna_wasm_fixturator = DnaWasmFixturator::new(Unpredictable);
        for _ in 0..number_of_wasms {
            let wasm = dna_wasm_fixturator.next().unwrap();
            wasms.insert(
                tokio_safe_block_on::tokio_safe_block_forever_on(
                    async { WasmHash::with_data(&wasm).await },
                )
                .into(),
                wasm,
            );
        }
        wasms
    };
    curve Predictable {
        let mut wasms: Wasms = BTreeMap::new();
        let mut dna_wasm_fixturator = DnaWasmFixturator::new_indexed(Predictable, get_fixt_index!());
        for _ in 0..3 {
            let wasm = dna_wasm_fixturator.next().unwrap();
            wasms.insert(
                tokio_safe_block_on::tokio_safe_block_forever_on(
                    async { WasmHash::with_data(&wasm).await },
                )
                .into(),
                wasm,
            );
        }
        wasms
    };
);

fixturator!(
    DnaFile,
    {
        DnaFile {
            dna: tokio_safe_block_on::tokio_safe_block_forever_on(async move {
                DnaDefHashed::from_content(DnaDefFixturator::new(Empty).next().unwrap()).await
            }),
            code: WasmsFixturator::new(Empty).next().unwrap(),
        }
    },
    {
        // align the wasm hashes across the file and def
        let mut zome_name_fixturator = ZomeNameFixturator::new(Unpredictable);
        let wasms = WasmsFixturator::new(Unpredictable).next().unwrap();
        let mut zomes: Zomes = Vec::new();
        for (hash, _) in wasms {
            zomes.push((
                zome_name_fixturator.next().unwrap(),
                WasmZome {
                    wasm_hash: hash.to_owned(),
                }
                .into(),
            ));
        }
        let mut dna_def = DnaDefFixturator::new(Unpredictable).next().unwrap();
        dna_def.zomes = zomes;
        let dna = tokio_safe_block_on::tokio_safe_block_forever_on(async move {
            DnaDefHashed::from_content(dna_def).await
        });
        DnaFile {
            dna,
            code: WasmsFixturator::new(Unpredictable).next().unwrap(),
        }
    },
    {
        // align the wasm hashes across the file and def
        let mut zome_name_fixturator =
            ZomeNameFixturator::new_indexed(Predictable, get_fixt_index!());
        let wasms = WasmsFixturator::new_indexed(Predictable, get_fixt_index!())
            .next()
            .unwrap();
        let mut zomes: Zomes = Vec::new();
        for (hash, _) in wasms {
            zomes.push((
                zome_name_fixturator.next().unwrap(),
                WasmZome {
                    wasm_hash: hash.to_owned(),
                }
                .into(),
            ));
        }
        let mut dna_def = DnaDefFixturator::new_indexed(Predictable, get_fixt_index!())
            .next()
            .unwrap();
        dna_def.zomes = zomes;
        let dna = tokio_safe_block_on::tokio_safe_block_forever_on(async move {
            DnaDefHashed::from_content(dna_def).await
        });
        DnaFile {
            dna,
            code: WasmsFixturator::new_indexed(Predictable, get_fixt_index!())
                .next()
                .unwrap(),
        }
    }
);

fixturator!(
    LinkMetaVal;
    constructor fn new(HeaderHash, EntryHash, Timestamp, u8, LinkTag);
);

impl Iterator for LinkMetaValFixturator<(EntryHash, LinkTag)> {
    type Item = LinkMetaVal;
    fn next(&mut self) -> Option<Self::Item> {
        let mut f = fixt!(LinkMetaVal);
        f.target = self.0.curve.0.clone();
        f.tag = self.0.curve.1.clone();
        Some(f)
    }
}

fixturator!(
    HeaderHashes,
    vec![].into(),
    {
        let mut rng = rand::thread_rng();
        let number_of_hashes = rng.gen_range(0, 5);

        let mut hashes: Vec<HeaderHash> = vec![];
        let mut header_hash_fixturator = HeaderHashFixturator::new(Unpredictable);
        for _ in 0..number_of_hashes {
            hashes.push(header_hash_fixturator.next().unwrap().into());
        }
        hashes.into()
    },
    {
        let mut hashes: Vec<HeaderHash> = vec![];
        let mut header_hash_fixturator =
            HeaderHashFixturator::new_indexed(Predictable, get_fixt_index!());
        for _ in 0..3 {
            hashes.push(header_hash_fixturator.next().unwrap().into());
        }
        hashes.into()
    }
);

fixturator!(
    KeystoreSender;
    curve Empty {
        tokio_safe_block_on::tokio_safe_block_forever_on(async {
            // an empty keystore
            holochain_keystore::test_keystore::spawn_test_keystore().await.unwrap()
        })
    };
    curve Unpredictable {
        // TODO: Make this unpredictable
        tokio_safe_block_on::tokio_safe_block_forever_on(async {
            holochain_keystore::test_keystore::spawn_test_keystore().await.unwrap()
        })
    };
    // a prepopulate keystore with hardcoded agents in it
    curve Predictable test_keystore();
);

fixturator!(
    SignalBroadcaster;
    curve Empty {
        SignalBroadcaster::new(Vec::new())
    };
    curve Unpredictable {
        SignalBroadcaster::new(Vec::new())
    };
    curve Predictable {
        SignalBroadcaster::new(Vec::new())
    };
);

fixturator!(
    CallZomeWorkspaceLock;
    curve Empty {
        // XXX: This may not be great to just grab an environment for this purpose.
        //      It is assumed that this value is never really used in any "real"
        //      way, because previously, it was implemented as a null pointer
        //      wrapped in an UnsafeZomeCallWorkspace
        let env = holochain_lmdb::test_utils::test_cell_env();
        CallZomeWorkspaceLock::new(CallZomeWorkspace::new(env.env().into()).unwrap())
    };
    curve Unpredictable {
        CallZomeWorkspaceLockFixturator::new(Empty)
            .next()
            .unwrap()
    };
    curve Predictable {
        CallZomeWorkspaceLockFixturator::new(Empty)
            .next()
            .unwrap()
    };
);

fn make_call_zome_handle(cell_id: CellId) -> CellConductorReadHandle {
    let handle = Arc::new(MockConductorHandleT::new());
    let cell_conductor_api = CellConductorApi::new(handle, cell_id);
    Arc::new(cell_conductor_api)
}

fixturator!(
    CellConductorReadHandle;
    vanilla fn make_call_zome_handle(CellId);
);

fixturator!(
    ZomeCallHostAccess;
    constructor fn new(CallZomeWorkspaceLock, KeystoreSender, HolochainP2pCell, SignalBroadcaster, CellConductorReadHandle, CellId);
);

fixturator!(
    EntryDefsInvocation;
    constructor fn new();
);

fixturator!(
    EntryDefsHostAccess;
    constructor fn new();
);

fixturator!(
    InitInvocation;
    constructor fn new(DnaDef);
);

fixturator!(
    InitHostAccess;
    constructor fn new(CallZomeWorkspaceLock, KeystoreSender, HolochainP2pCell);
);

fixturator!(
    MigrateAgentInvocation;
    constructor fn new(DnaDef, MigrateAgent);
);

fixturator!(
    MigrateAgentHostAccess;
    constructor fn new(CallZomeWorkspaceLock);
);

fixturator!(
    PostCommitInvocation;
    constructor fn new(Zome, HeaderHashes);
);

fixturator!(
    PostCommitHostAccess;
    constructor fn new(CallZomeWorkspaceLock, KeystoreSender, HolochainP2pCell);
);

fixturator!(
    ZomesToInvoke;
    constructor fn one(Zome);
);

fn make_validate_invocation(
    zomes_to_invoke: ZomesToInvoke,
    element: Element,
) -> ValidateInvocation {
    ValidateInvocation {
        zomes_to_invoke,
        element: Arc::new(element),
        validation_package: None,
        entry_def_id: None,
    }
}

fixturator!(
    ValidateInvocation;
    vanilla fn make_validate_invocation(ZomesToInvoke, Element);
);

fixturator!(
    ValidateCreateLinkInvocation;
    constructor fn new(Zome, CreateLink, Entry, Entry);
);

fixturator!(
    ValidateDeleteLinkInvocation;
    constructor fn new(Zome, DeleteLink);
);

/// Macros don't get along with generics.
type ValidateLinkInvocationCreate = ValidateLinkInvocation<ValidateCreateLinkInvocation>;

fixturator!(
    ValidateLinkInvocationCreate;
    constructor fn new(ValidateCreateLinkInvocation);
    curve Zome {
        let mut c = ValidateCreateLinkInvocationFixturator::new(Empty)
            .next()
            .unwrap();
        c.zome = get_fixt_curve!().clone();
        ValidateLinkInvocationCreate::new(c)
    };
);

/// Macros don't get along with generics.
type ValidateLinkInvocationDelete = ValidateLinkInvocation<ValidateDeleteLinkInvocation>;

fixturator!(
    ValidateLinkInvocationDelete;
    constructor fn new(ValidateDeleteLinkInvocation);
);

fixturator!(
    ValidateLinkHostAccess;
    constructor fn new(CallZomeWorkspaceLock, HolochainP2pCell);
);

fixturator!(
    ValidateHostAccess;
    constructor fn new(CallZomeWorkspaceLock, HolochainP2pCell);
);

fixturator!(
    ValidationPackageInvocation;
    constructor fn new(Zome, AppEntryType);
);

fixturator!(
    ValidationPackageHostAccess;
    constructor fn new(CallZomeWorkspaceLock, HolochainP2pCell);
);

fixturator!(
    HostAccess;
    variants [
        ZomeCall(ZomeCallHostAccess)
        Validate(ValidateHostAccess)
        Init(InitHostAccess)
        EntryDefs(EntryDefsHostAccess)
        MigrateAgent(MigrateAgentHostAccess)
        ValidationPackage(ValidationPackageHostAccess)
        PostCommit(PostCommitHostAccess)
    ];
);

fixturator!(
    CallContext;
    constructor fn new(Zome, HostAccess);
);
