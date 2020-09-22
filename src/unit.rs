use bevy::prelude::*;
use bevy_rapier3d::rapier::{
    geometry::ColliderBuilder,
    dynamics::RigidBodyBuilder,
};
use bevy_mod_picking::PickableMesh;

use crate::mesh_picking::*;

// type AsBundle<B> = (B,);

// impl BundleFlatten for Unit
// {
//     type BundleSet = (PbrComponents, (ColliderBuilder,), (RigidBodyBuilder,), (PickableMesh,));

//     fn bundles(self) -> Self::BundleSet
//     {
//         (self.shape, (self.collider,), (self.rigid_body,), (self.picker,))
//     }
// }

// #[derive(Bundle)]
pub struct Unit
{
    pub shape: PbrComponents,
    pub collider: ColliderBuilder,
    pub rigid_body: RigidBodyBuilder,
    pub picker: PickableMesh
}

impl Default for Unit
{
    fn default() -> Self 
    {
        Unit {
            shape: PbrComponents::default(),
            collider: ColliderBuilder::cuboid(0.0, 0.0, 0.0),
            rigid_body: RigidBodyBuilder::new_dynamic(),
            picker: PickableMesh::empty(),
        }
    }
}

pub trait FlattenBundle<T=Commands>
where
    Self: Sized,
{
    fn spawn_flattened(self, spawner: &mut T) -> &mut T;
}

pub struct AsFlat<F: FlattenBundle>(pub F);
impl<F: FlattenBundle> FlattenBundle for AsFlat<F>
{
    fn spawn_flattened(self, spawner: &mut Commands) -> &mut Commands
    {
        self.0.spawn_flattened(spawner)
    }
}

impl FlattenBundle for Unit
{
    fn spawn_flattened(self, spawner: &mut Commands) -> &mut Commands
    {
        spawner
            .spawn(self.shape)
            .with(self.collider)
            .with(self.rigid_body)
            .with(self.picker)
    }
}

pub trait FlatSpawn
{
    fn spawn_flat(&mut self, components: impl FlattenBundle + Component) -> &mut Self;
}

impl FlatSpawn for Commands
{
    fn spawn_flat(&mut self, components: impl FlattenBundle + Component) -> &mut Self
    {
        components.spawn_flattened(self)
    }
}
