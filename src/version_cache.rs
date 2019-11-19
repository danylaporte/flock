use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::{cell::Cell, ops::Deref, sync::Arc};
use thread_local::ThreadLocal;
use version_tag::{combine, VersionTag};

pub struct VersionCache<T: Send + Sync>(OnceCell<Inner<T>>);

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
        self.0
            .get_or_init(Inner::new)
            .get_or_init(init, combine(tags))
    }
}

pub struct CacheData<T>(Arc<(T, VersionTag)>);

impl<T> CacheData<T> {
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

    fn deref(&self) -> &Self::Target {
        &(self.0).0
    }
}

struct Inner<T>
where
    T: Send + Sync,
{
    local: ThreadLocal<Cell<Option<CacheData<T>>>>,
    mutex: Mutex<Option<CacheData<T>>>,
}

impl<T> Inner<T>
where
    T: Send + Sync,
{
    fn new() -> Self {
        Self {
            local: ThreadLocal::new(),
            mutex: Mutex::new(None),
        }
    }

    fn get_or_init<F>(&self, init: F, tag: VersionTag) -> CacheData<T>
    where
        F: FnOnce() -> T,
    {
        let cell = self.local.get_or(|| Cell::new(None));

        match cell.take() {
            Some(v) if v.tag() == tag => {
                cell.set(Some(v.clone()));
                return v;
            }
            Some(v) if v.tag() > tag => {
                cell.set(Some(v));
                init_lock(&self.mutex, init, tag)
            }
            _ => {
                let v = init_lock(&self.mutex, init, tag);
                cell.set(Some(v.clone()));
                v
            }
        }
    }
}

fn init_lock<T, F>(mutex: &Mutex<Option<CacheData<T>>>, init: F, tag: VersionTag) -> CacheData<T>
where
    F: FnOnce() -> T,
{
    let mut guard = mutex.lock();

    let guard_version = match &*guard {
        Some(v) if v.tag() == tag => {
            return v.clone();
        }
        Some(v) => Some(v.tag()),
        None => None,
    };

    let data = CacheData(Arc::new((init(), tag)));

    if guard_version.map_or(true, |v| v < tag) {
        *guard = Some(data.clone());
    }

    data
}

#[test]
fn cache_update_only_on_higher_version() {
    let old_dep = VersionTag::new();
    let new_dep = VersionTag::new();
    let cache = VersionCache::new();

    // cache will be created using the new dependency first
    let data = cache.get_or_init(|| 1, &[new_dep]);
    assert_eq!((1, new_dep), (*data, data.tag()));

    // cache should should return the old dependency value
    let data = cache.get_or_init(|| 0, &[old_dep]);
    assert_eq!((0, old_dep), (*data, data.tag()));

    let new_dep = VersionTag::new();

    // cache should be updated now
    let data = cache.get_or_init(|| 2, &[new_dep]);
    assert_eq!((2, new_dep), (*data, data.tag()));
}
