use std::marker::PhantomData;
use uuid::Uuid;

pub type AccountId = EntityId<u32, Uuid, AccounIdSet>;

pub struct ComposedKey(u32);

pub struct EntityId<UID, UUID, SET>(UID, PhantomData<UUID>, PhantomData<SET>);

impl<UID, UUID, SET> Deserialize for EntityId<UID, UUID, SET> {

}

// cmp
// debug
// deserialize
// display
// eq
// grid_serialize
// into
// serialize

// permet de tagger des u16, u32, u64 pour leur permettre d'être géré dans un entity id
trait UId {}
