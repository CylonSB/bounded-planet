use bevy::prelude::*;

/// Convert a screen point into a NDC (Normalized Device Coordinate) point.
/// 
/// Uses screen size as part of the conversion math.
pub fn screen_to_ndc(point: Vec2, screen_size: Vec2) -> Vec3 {
    ((point / screen_size) * 2.0 - Vec2::new(1.0, 1.0)).extend(0.0)
}

/// Convert a NDC (Normalized Device Coordinate) point into a world point.
/// 
/// Uses the camera transform matrix (preferably global) and the camera projection matrix as part of the conversion math.
pub fn ndc_to_world(point: Vec3, camera_transform: Mat4, projection_matrix: Mat4) -> Vec3 {
    let matrix = camera_transform * projection_matrix.inverse();
    matrix.transform_point3(point)
}

/// Convert a screen point into a world point by calling [`screen_to_ndc`] followed by [`ndc_to_world`].
/// 
/// Uses the screen size, camera transform matrix (preferably global), and camera projection matrix as part of the conversion math.
pub fn screen_to_world(point: Vec2,  screen_size: Vec2, camera_transform: Mat4, projection_matrix: Mat4) -> Vec3 {
    let point = screen_to_ndc(point, screen_size);
    ndc_to_world(point, camera_transform, projection_matrix)
}