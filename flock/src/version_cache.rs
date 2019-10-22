use arc_swap::{ArcSwapAny, Guard};
use once_cell::sync::OnceCell;
use std::sync::Arc;
use version_tag::{combine, VersionTag};

pub type ArcData<T> = Arc<(T, VersionTag)>;
type ArcSwapData<T> = ArcSwapAny<ArcData<T>>;

pub struct VersionCache<T>(OnceCell<ArcSwapData<T>>);

impl<T> VersionCache<T> {
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    #[cfg(test)]
    fn get(&self) -> Option<ArcData<T>> {
        self.0.get().map(|swap| swap.load_full())
    }

    pub fn get_or_init<F>(&self, f: F, tags: &[VersionTag]) -> ArcData<T>
    where
        F: FnOnce() -> T,
    {
        /// Update the arc in the arc_swap struct.
        fn update<'a, U>(
            arc_swap: &'a ArcSwapData<U>,
            mut guard: Guard<'a, ArcData<U>>,
            new: ArcData<U>,
        ) -> ArcData<U> {
            loop {
                guard = arc_swap.compare_and_swap(guard, new.clone());

                // check the version, if it matches the actual version, keep it
                if guard.1 >= new.1 {
                    return Guard::into_inner(guard);
                }
            }
        }

        // compute the new version tag based on dependencies
        let version = combine(tags);

        // check if the cell is already filled
        if let Some(swap) = self.0.get() {
            let guard = swap.load();

            if guard.1 == version {
                return Guard::into_inner(guard);
            }

            let new = Arc::new((f(), version));

            return if guard.1 > version {
                new
            } else {
                update(swap, guard, new)
            };
        }

        let swap = ArcSwapAny::new(Arc::new((f(), version)));

        match self.0.set(swap) {
            Ok(_) => self.0.get().expect("once_cell").load_full(),
            Err(swap) => {
                let new = swap.into_inner();
                let swap = self.0.get().expect("once_cell");
                let current = swap.load();
                update(swap, current, new)
            }
        }
    }
}

#[test]
fn cache_update_only_on_higher_version() {
    let old_dep = VersionTag::new();
    let new_dep = VersionTag::new();
    let cache = VersionCache::new();

    // cache will be created using the new dependency first
    assert_eq!((1, new_dep), *cache.get_or_init(|| 1, &[new_dep]));

    // cache should should return the old dependency value
    assert_eq!((0, old_dep), *cache.get_or_init(|| 0, &[old_dep]));

    // cache should still be using the new value
    assert_eq!((1, new_dep), *cache.get().expect("value"));

    let new_dep = VersionTag::new();

    // cache should be updated now
    assert_eq!((2, new_dep), *cache.get_or_init(|| 2, &[new_dep]));
    assert_eq!((2, new_dep), *cache.get().expect("value"));
}
