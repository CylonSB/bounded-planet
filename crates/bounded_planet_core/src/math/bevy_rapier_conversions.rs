use bevy::prelude::*;

use bevy_rapier3d::{
    rapier::math::{
        Point,
        Vector
    }
};

/// Custom conversion trait for turning Bevy (or other crate) math types into Rapier math types.
/// 
/// Converts Self into T.
pub trait IntoRapierMath<T> {
    /// Convert Self into T
    fn into_rapier(self) -> T;
}

impl IntoRapierMath<Point<f32>> for Vec3 {
    /// Convert Bevy Vec3 into Rapier Point<f32>
    fn into_rapier(self) -> Point<f32> {
        Point::new(self.x(), self.y(), self.z())
    }
}

impl IntoRapierMath<Vector<f32>> for Vec3 {
    /// Convert Bevy Vec3 into Rapier Vector<f32>
    fn into_rapier(self) -> Vector<f32> {
        Vector::new(self.x(), self.y(), self.z())
    }
}