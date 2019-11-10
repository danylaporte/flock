pub(crate) enum ChangeState<ENTITY> {
    Inserted(ENTITY),
    Removed,
    Updated(ENTITY),
}

impl<ENTITY> ChangeState<ENTITY> {
    pub fn is_insert_or_update(&self) -> bool {
        match self {
            Self::Inserted(_) | Self::Updated(_) => true,
            Self::Removed => false,
        }
    }
}
