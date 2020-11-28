use crate::element_buf::ElementBuf;
use crate::metadata::MetadataBuf;
use crate::metadata::MetadataBufT;
use holochain_lmdb::env::EnvironmentRead;
use holochain_lmdb::prelude::AuthoredPrefix;
use holochain_lmdb::prelude::IntegratedPrefix;
use holochain_lmdb::prelude::PendingPrefix;
use holochain_lmdb::prelude::PrefixType;
use holochain_lmdb::prelude::RejectedPrefix;

pub struct CascadeLocal<
    'a,
    MetaVault = MetadataBuf,
    MetaAuthored = MetadataBuf<AuthoredPrefix>,
    MetaCache = MetadataBuf,
    MetaPending = MetadataBuf<PendingPrefix>,
    MetaRejected = MetadataBuf<RejectedPrefix>,
> where
    MetaVault: MetadataBufT,
    MetaAuthored: MetadataBufT<AuthoredPrefix>,
    MetaPending: MetadataBufT<PendingPrefix>,
    MetaRejected: MetadataBufT<RejectedPrefix>,
    MetaCache: MetadataBufT,
{
    integrated_data: Option<DbPair<'a, MetaVault, IntegratedPrefix>>,
    authored_data: Option<DbPair<'a, MetaAuthored, AuthoredPrefix>>,
    pending_data: Option<DbPair<'a, MetaPending, PendingPrefix>>,
    rejected_data: Option<DbPair<'a, MetaRejected, RejectedPrefix>>,
    cache_data: Option<DbPairMut<'a, MetaCache>>,
    env: Option<EnvironmentRead>,
}

/// A pair containing an element buf and metadata buf
/// with the same prefix.
/// The default IntegratedPrefix is for databases that don't
/// actually use prefixes (like the cache). In this case we just
/// choose the first one (IntegratedPrefix)
#[derive(derive_more::Constructor)]
pub struct DbPair<'a, M, P = IntegratedPrefix>
where
    P: PrefixType,
    M: MetadataBufT<P>,
{
    pub element: &'a ElementBuf<P>,
    pub meta: &'a M,
}

#[derive(derive_more::Constructor)]
pub struct DbPairMut<'a, M, P = IntegratedPrefix>
where
    P: PrefixType,
    M: MetadataBufT<P>,
{
    pub element: &'a mut ElementBuf<P>,
    pub meta: &'a mut M,
}
