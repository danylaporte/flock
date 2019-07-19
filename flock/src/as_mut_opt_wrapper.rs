use crate::AsMutOpt;

pub struct AsMutOptWrapper<T>(Option<T>);

impl<T> AsMutOptWrapper<T> {
    pub fn new(value: T) -> Self {
        Self(Some(value))
    }
}

impl<T> AsMutOptWrapper<T> {
    pub fn into_inner(self) -> T {
        self.0.expect("AsMutOptWrapper")
    }
}

impl<T> AsMutOpt<T> for AsMutOptWrapper<T> {
    fn as_mut_opt(&mut self) -> Option<&mut T> {
        self.0.as_mut()
    }
}

impl<T> AsRef<Option<T>> for AsMutOptWrapper<T> {
    fn as_ref(&self) -> &Option<T> {
        &self.0
    }
}
