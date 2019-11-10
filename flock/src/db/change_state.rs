pub(crate) enum ChangeState<ENTITY> {
    Inserted(ENTITY),
    Removed,
    Updated(ENTITY),
}
