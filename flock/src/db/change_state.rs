pub(crate) enum ChangeState<ENTITY> {
    Removed,
    Updated(Box<ENTITY>),
    Unchanged,
}
