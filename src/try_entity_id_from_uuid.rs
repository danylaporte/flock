use crate::Uuid;

pub trait TryEntityIdFromUuid: Sized {
    fn try_entity_id_from_uuid(u: Uuid) -> Option<Self>;
}
