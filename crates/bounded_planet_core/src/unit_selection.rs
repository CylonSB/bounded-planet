use bevy::{
    prelude::*,
    render::{
        render_graph::base,
        camera::Camera
    }
};

use bevy_rapier3d::{
    physics::ColliderHandleComponent,
    rapier::{
        geometry::{
            ColliderSet,
            Ray,
        },
        pipeline::QueryPipeline,
    }
};

use crate::math::{self, IntoRapierMath};

/// Enables selection of entities with rapier colliders based on location of mouse upon left click.
pub struct UnitSelectionPlugin;

impl Plugin for UnitSelectionPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_event::<SelectionEvent>()
            .init_resource::<SelectionState>()
            .add_system(selection_update.system())
        ;
    }
}

// TODO(#68): rather than managing selection with events, it should be component-oriented
/// Represents the selection of a given entity, or that nothing was selected
#[derive(Debug, Clone)]
pub struct SelectionEvent {
    selected: Option<Entity>
}

/// State of selected objects managed by the [`UnitSelectionPlugin`]
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub currently_selected: Option<Entity>,
    pub last_selected: Option<Entity>,
}

impl SelectionState {
    /// Swaps the currently selected to the last selected, then clears the currently selected
    pub fn swap_and_clear_current(&mut self) {
        self.last_selected = self.currently_selected;
        self.currently_selected = None;
    }
}

/// Local state of [`selection_update`] system
#[derive(Default)]
struct SelectionUpdateState {
    /// Event reader for [`CursorMoved`] events, providing the current screen position of the cursor.
    pub cursor_moved_reader: EventReader<CursorMoved>,
    /// Current position of the cursor on the screen. Is updated every time a [`CursorMoved`] even fires.
    pub cursor_position: Vec2,
}

/// System for all the logic related to unit selection.
/// 
/// When the cursor moves, update the `cursor_position` field in [`SelectionUpdateState`]. Then if the user left clicks,
/// deselects the currently selected entity and translates the cursor position into a world point and uses it to cast a ray into the world.
/// Then sends a [`SelectionEvent`] with the entity that has been hit, or nothing.
#[allow(clippy::too_many_arguments)]
fn selection_update(
    mut state: Local<SelectionUpdateState>,
    mut selection_state: ResMut<SelectionState>,

    mut selection_events: ResMut<Events<SelectionEvent>>,
    cursor_moved: Res<Events<CursorMoved>>,

    query_pipeline: Res<QueryPipeline>,
    colliders: Res<ColliderSet>,
    mouse_button_inputs: Res<Input<MouseButton>>,

    windows: Res<Windows>,

    mut camera_query: Query<(&Camera, &GlobalTransform)>,
    mut colliders_query: Query<(Entity, &ColliderHandleComponent)>,
) {
    // Save the current cursor position every time it moves. If this is only done when the mouse button is pressed
    // then you'll often read no cursor moved events since it only holds the event for 2 ticks max.
    if let Some(CursorMoved { position, .. }) = state.cursor_moved_reader.latest(&cursor_moved) {
        state.cursor_position = *position;
    }
    
    // TODO(#65): when a proper input handler exists, use it for the unit selection functionality
    if mouse_button_inputs.just_pressed(MouseButton::Left) {
        // If the user clicks at all, clear the selection state. Aka, deselect whatever is selected
        selection_state.swap_and_clear_current();
        
        // TODO(#64): properly figure out _which_ camera was intended to be used
        // Search for a camera with the default 3d camera name, which should be the "main" camera in a 3d game
        let mut camera_query_iter = camera_query.iter();
        let (camera, camera_transform) = camera_query_iter.into_iter()
            .find(|&(camera, _)| {
                // If Some(name), return name == base::camera::CAMERA3D. If None, return false
                camera.name.as_ref().map_or(false, |name| name == base::camera::CAMERA3D)
            })
            .expect("Unable to find a main camera! Do you not have a Camera3dComponents in use?");

        let window = windows.get(camera.window)
            .expect("Unable to find the window of the 3d camera! Are you running headless yet trying to do screen-based unit selection?");
        let screen_size = Vec2::new(window.width as f32, window.height as f32);

        // Turn the cursor position on the screen into a position in world space (but still on the camera plane)
        let point = math::screen_to_world(
            state.cursor_position,
            screen_size,
            *camera_transform.value(),
            camera.projection_matrix,
        );

        // Construct a direction vector starting from the camera's location in world space, passing through the cursor position in world space
        let direction = point - camera_transform.translation();
        // Construct the ray starting at the screen point in world space, heading in the computed direction
        let ray = Ray::new(point.into_rapier(), direction.into_rapier());

        if let Some((hit_collider, _, _)) = query_pipeline.cast_ray(&colliders, &ray, std::f32::MAX) {
            // The raycast hit something!

            // TODO: #67 we should have a real map between entity <-> collider handle used in unit selection
            // Search for an entity with the same collider handle as the collider we hit
            let mut colliders_query_iter = colliders_query.iter();
            let query_search = colliders_query_iter.into_iter()
                .find(|&(_, collider_handle)| {
                    collider_handle.handle() == hit_collider
                });

            let (collided_entity, _) = query_search
                .expect("There was a raycast that hit an entity, but we can't find it in a query with collider handles! Somehow it got lost...");

            selection_state.currently_selected = Some(collided_entity);
            selection_events.send(SelectionEvent {
                selected: Some(collided_entity)
            });
        } else {
            // If there was no raycast collision :(

            selection_state.currently_selected = None;
            selection_events.send(SelectionEvent {
                selected: None
            });
        }
    }
}

/// Simple debug plugin that changes the material color of selected entities to the specified color, to help seeing what's selected.
#[derive(Debug)]
pub struct UnitSelectionHighlighterPlugin;

impl Plugin for UnitSelectionHighlighterPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system(highlight_selected.system());
    }
}

/// Local state of [`highlight_selected`] system.
#[derive(Default)]
struct SelectionHighlighterState {
    /// Event reader for [`SelectionEvent`] events
    event_reader: EventReader<SelectionEvent>,
    /// Color to highlight entities with when they are selected.
    highlight_color: Color,
    /// The original color of the last selected entity. Used to restore its original color when it's deselected.
    last_selected_color: Option<Color>,
}

/// When a unit is selected, highlight it with a configured highlight color. When that unit is deselected, revert the color to its original.
/// 
/// This is to serve as a simple debug renderer to help illustrate selected objects.
fn highlight_selected(
    mut state: Local<SelectionHighlighterState>,
    selection_state: Res<SelectionState>,
    selection_events: Res<Events<SelectionEvent>>,

    mut materials: ResMut<Assets<StandardMaterial>>,

    query: Query<(&mut Handle<StandardMaterial>,)>,
) {
    if let Some(event) = state.event_reader.latest(&selection_events) {
        // Try to reset the last selected entity back to its original color
        if let Some(entity) = selection_state.last_selected {
            if let Some(color) = state.last_selected_color {
                let handle = query.get_mut::<Handle<StandardMaterial>>(entity).unwrap();
                materials.get_mut(&handle).unwrap().albedo = color;
            }
        }

        // If another entity was actually selected, set it as highlighted
        if let Some(entity) = event.selected {
            let handle = query.get_mut::<Handle<StandardMaterial>>(entity).unwrap();
            let material = materials.get_mut(&handle).unwrap();


            println!("Current material color: {:?}, new highlight color: {:?}", material.albedo, state.highlight_color);
            state.last_selected_color = Some(material.albedo);
            material.albedo = state.highlight_color;
        }

    }
}
