use bevy::prelude::*;

pub fn screen_to_camera(point: Vec2, projection_matrix: Mat4) -> Vec3 {
    transform_point(point.extend(1.0), projection_matrix.inverse())
}

pub fn camera_to_world(point: Vec3, camera_transform: Transform) -> Vec3 {
    transform_point(point, camera_transform.value.inverse())
}

fn transform_point(point: Vec3, matrix: Mat4) -> Vec3 {
    let point_vec4 = point.extend(1.0);
    let transformed_point = matrix.mul_vec4(point_vec4);

    transformed_point.truncate().into()
}
