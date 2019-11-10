use std::ops::Deref;

pub struct EntityRef<CTX, ENTITY> {
    pub ctx: CTX,
    pub(crate) entity: ENTITY,
}

impl<CTX, ENTITY> Deref for EntityRef<CTX, ENTITY> {
    type Target = ENTITY;

    fn deref(&self) -> &Self::Target {
        &self.entity
    }
}

pub struct EntityRefIter<CTX, I> {
    ctx: CTX,
    iter: I,
}

impl<CTX, I> Iterator for EntityRefIter<CTX, I>
where
    I: Iterator,
    CTX: Copy,
{
    type Item = EntityRef<CTX, I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|entity| EntityRef {
            ctx: self.ctx,
            entity,
        })
    }
}

// doit être généré
pub struct DbCtx<TABLES, RELATIONS> {
    relations: RELATIONS,
    tables: TABLES,
}

impl<TABLES, RELATIONS> DbCtx<TABLES, RELATIONS> {
    pub fn relations(&self) -> &RELATIONS {
        &self.relations
    }
}

impl<TABLES, RELATIONS> Deref for DbCtx<TABLES, RELATIONS> {
    type Target = TABLES;

    fn deref(&self) -> &Self::Target {
        &self.tables
    }
}
