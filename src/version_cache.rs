use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use std::{cell::Cell, ops::Deref, sync::Arc};
use thread_local::CachedThreadLocal;
use version_tag::{combine, VersionTag};

pub struct VersionCache<T: Send + Sync>(OnceCell<(RwLock<Option<CacheData<T>>>, ReentrencyCheck)>);

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
        let (lock, reentrency) = self
            .0
            .get_or_init(|| (RwLock::new(None), ReentrencyCheck::new()));

        reentrency.check_and_panic();

        let tag = combine(tags);

        // ensure drop read lock before acquiring the write
        {
            if let Some(cache_data) = lock.read().as_ref().filter(|v| v.tag() == tag) {
                return cache_data.clone();
            }
        }

        let mut opt = lock.write();

        if let Some(cache_data) = opt.as_ref().filter(|v| v.tag() == tag) {
            return cache_data.clone();
        }

        // block the initialization for this thread.
        let _reentrency_guard = reentrency.only_once();

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

struct ReentrencyCheck(CachedThreadLocal<Cell<bool>>);

impl ReentrencyCheck {
    fn new() -> Self {
        Self(CachedThreadLocal::new())
    }

    fn check_and_panic(&self) {
        if self.0.get_or_default().get() {
            panic!("Reentrency prohibited");
        }
    }

    fn only_once(&self) -> ReentrencyGuard {
        self.0.get_or_default().set(true);
        ReentrencyGuard(self)
    }
}

struct ReentrencyGuard<'a>(&'a ReentrencyCheck);

impl<'a> Drop for ReentrencyGuard<'a> {
    fn drop(&mut self) {
        (self.0).0.get_or_default().set(false)
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

#[test]
fn deadlock_test() {
    use std::{thread, time::Duration};

    let cache = Arc::new(VersionCache::new());
    let mut threads = Vec::with_capacity(4);

    for i in 0..4 {
        let cache = cache.clone();
        threads.push(thread::spawn(move || {
            let tag = VersionTag::new();
            let dur = Duration::from_millis(i * 10);
            for _ in 0..100 {
                cache.get_or_init(
                    || {
                        thread::sleep(dur);
                        1
                    },
                    &[tag],
                );
            }
        }));
    }

    for t in threads {
        let _ = t.join();
    }
}
