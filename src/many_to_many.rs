use crate::iter::ManyIter;
use std::iter::FromIterator;

pub struct ManyToMany<L, R> {
    left: Vec<Vec<L>>,
    right: Vec<Vec<R>>,
}

impl<L, R> ManyToMany<L, R> {
    pub fn contains_left_right(&self, l: L, r: R) -> bool
    where
        L: Clone + Into<usize> + Ord,
        R: Clone + Into<usize> + Ord,
    {
        self.left
            .get(r.clone().into())
            .and_then(|left| {
                let right = self.right.get(l.clone().into())?;

                Some(if left.len() < right.len() {
                    left.binary_search(&l).is_ok()
                } else {
                    right.binary_search(&r).is_ok()
                })
            })
            .unwrap_or(false)
    }

    pub fn iter_left_by(&self, id: R) -> ManyIter<L>
    where
        L: Clone,
        R: Into<usize>,
    {
        ManyIter(
            self.left
                .get(id.into())
                .map(|v| v.as_slice())
                .unwrap_or(&[])
                .into_iter(),
        )
    }

    pub fn iter_right_by(&self, id: L) -> ManyIter<R>
    where
        L: Into<usize>,
        R: Clone,
    {
        ManyIter(
            self.right
                .get(id.into())
                .map(|v| v.as_slice())
                .unwrap_or(&[])
                .into_iter(),
        )
    }
}

impl<L, R> FromIterator<(L, R)> for ManyToMany<L, R>
where
    L: Into<usize> + Clone + Ord + PartialEq + Send,
    R: Into<usize> + Clone + Ord + PartialEq + Send,
{
    fn from_iter<T: IntoIterator<Item = (L, R)>>(iter: T) -> Self {
        let mut left = Vec::new();
        let mut right = Vec::new();

        fn add<T>(v: &mut Vec<Vec<T>>, idx: usize, item: T) {
            v.extend((v.len()..=idx).into_iter().map(|_| Vec::new()));
            v[idx].push(item);
        }

        for (l, r) in iter {
            let idx_l = l.clone().into();
            let idx_r = r.clone().into();
            add(&mut left, idx_r, l);
            add(&mut right, idx_l, r);
        }

        fn dedup<T>(v: &mut Vec<Vec<T>>)
        where
            T: Ord + Send,
        {
            v.iter_mut().for_each(|v| {
                v.sort_unstable();
                v.dedup();
                v.shrink_to_fit();
            });
            v.shrink_to_fit();
        }

        dedup(&mut left);
        dedup(&mut right);

        //join(|| dedup(&mut left), || dedup(&mut right));

        Self { left, right }
    }
}

#[test]
fn test_contains_left_right() {
    let m2m = std::iter::once((1, 3)).collect::<ManyToMany<usize, usize>>();

    assert!(m2m.contains_left_right(1, 3));
    assert!(!m2m.contains_left_right(3, 1));
    assert!(!m2m.contains_left_right(2, 2));
    assert!(!m2m.contains_left_right(2, 5));
}

#[test]
fn test_many_to_many_from_iterator() {
    let _ = std::iter::once((1, 3)).collect::<ManyToMany<usize, usize>>();
}
