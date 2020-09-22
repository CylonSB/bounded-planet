use bevy::prelude::Entity;
use bevy_mod_picking::*;

pub trait PickableMeshExt
{
    fn empty() -> Self;
}

impl PickableMeshExt for PickableMesh
{
    fn empty() -> Self {
        PickableMesh::new(Entity::new())
    }
}