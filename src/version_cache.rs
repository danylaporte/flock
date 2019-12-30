use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::{ops::Deref, sync::Arc};
use version_tag::{combine, VersionTag};

pub struct VersionCache<T: Send + Sync>(OnceCell<RwLock<Option<CacheData<T>>>>);

impl<T> VersionCache<T>
where
    T: Send + Sync,
{
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    pub fn get_or_init<F>(&self, init: F, tags: &[VersionTag]) -> CacheData<T>
    where
        T: Send + Sync,
        F: FnOnce() -> T,
    {
        let tag = combine(tags);
        let lock = self.0.get_or_init(|| RwLock::new(None));

        if let Some(cache_data) = lock.read().as_ref().filter(|v| v.tag() == tag) {
            return cache_data.clone();
        }

        let mut opt = lock.write();

        if let Some(cache_data) = opt.as_ref().filter(|v| v.tag() == tag) {
            return cache_data.clone();
        }

        let data = init();
        let cache_data = CacheData::new(data, tag);

        *opt = Some(cache_data.clone());

        cache_data
    }
}

pub struct CacheData<T>(Arc<(T, VersionTag)>);

impl<T> CacheData<T> {
    #[inline]
    fn new(data: T, tag: VersionTag) -> Self {
        CacheData(Arc::new((data, tag)))
    }

    #[inline]
    pub fn tag(&self) -> VersionTag {
        (self.0).1
    }
}

impl<T> Clone for CacheData<T> {
    fn clone(&self) -> Self {
        CacheData(Arc::clone(&self.0))
    }
}

impl<T> Deref for CacheData<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &(self.0).0
    }
}

#[test]
fn it_works() {
    let old_dep = VersionTag::new();
    let new_dep = VersionTag::new();
    let cache = VersionCache::new();

    // cache will be created using the new dependency first
    let data = cache.get_or_init(|| 1, &[old_dep]);
    assert_eq!((1, old_dep), (*data, data.tag()));

    // cache should be updated now
    let data = cache.get_or_init(|| 2, &[new_dep]);
    assert_eq!((2, new_dep), (*data, data.tag()));
}
