use crate::iter::ManyIter;
use rayon::{
    iter::{IntoParallelRefMutIterator, ParallelIterator},
    join,
};
use std::iter::FromIterator;

pub struct ManyToMany<L, R> {
    left: Vec<Vec<L>>,
    right: Vec<Vec<R>>,
}

impl<L, R> ManyToMany<L, R> {
    pub fn iter_left_by(&self, id: R) -> ManyIter<L>
    where
        L: Clone,
        R: Into<usize>,
    {
        ManyIter(
            self.left
                .get(id.into())
                .map(|v| &v[..])
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
                .map(|v| &v[..])
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
            v.par_iter_mut().for_each(|v| {
                v.sort_unstable();
                v.dedup();
                v.shrink_to_fit();
            });
            v.shrink_to_fit();
        }

        join(|| dedup(&mut left), || dedup(&mut right));

        Self { left, right }
    }
}

#[test]
fn test_many_to_many_from_iterator() {
    std::iter::once((1, 3)).collect::<ManyToMany<usize, usize>>();
}
