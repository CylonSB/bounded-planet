use bevy::prelude::*;
use bevy_mod_picking::*;

#[derive(Debug)]
pub struct SelectionPlugin;

impl Plugin for SelectionPlugin
{
    fn build(&self, app: &mut AppBuilder)
    {
        app
            .add_event::<SelectionEvent>()
            .init_resource::<SelectionState>()
            .init_resource::<SelectionColor>()
            .init_resource::<SelectionEventReader>()
            .add_system(selection_update.system())
            .add_system(highlight_selected.system());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Selectable;

#[derive(Debug, Clone)]
pub struct SelectionEvent {
    selected: Entity
}

#[derive(Default)]
pub struct SelectionEventReader {
    reader: EventReader<SelectionEvent>
}

#[derive(Debug, Clone, Default)]
pub struct SelectionState
{
    pub currently_selected: Option<Entity>,
    pub last_selected: Option<Entity>,
}

impl SelectionState
{
    /// Swaps the currently selected to the last selected, and clears the currently selected.
    pub fn swap_and_clear(&mut self)
    {
        self.current_to_last();
        self.currently_selected = None;
    }

    /// Sets the currently selected to the last selected.
    pub fn current_to_last(&mut self)
    {
        self.last_selected = self.currently_selected;
    }
}

pub struct SelectionColor(pub Color);

impl SelectionColor
{
    const SELECTED_COLOR: Color = Color::rgb(0.3, 0.8, 0.5);
}

impl Default for SelectionColor
{
    fn default() -> Self {
        SelectionColor(Self::SELECTED_COLOR)
    }
}


fn selection_update(
    pick_state: Res<PickState>,
    mut selection_state: ResMut<SelectionState>,
    mut selection_events: ResMut<Events<SelectionEvent>>,
    mouse_button_inputs: Res<Input<MouseButton>>,
    query: Query<&Selectable>
)
{
    if mouse_button_inputs.just_pressed(MouseButton::Left)
    {
        if let Some(pick_depth) = pick_state.top()
        {
            selection_state.swap_and_clear();
            
            if let Ok(_) = query.get::<Selectable>(pick_depth.entity())
            {
                selection_state.currently_selected = Some(pick_depth.entity());
                selection_events.send(SelectionEvent {
                    selected: pick_depth.entity()
                })
            }
        }
    }
}

#[derive(Default)]
struct LastSelectedColor(pub Color);

fn highlight_selected(
    mut local_last_selected_color: Local<LastSelectedColor>,
    mut selection_reader: ResMut<SelectionEventReader>,
    selection_events: Res<Events<SelectionEvent>>,
    selection_state: Res<SelectionState>,
    selection_color: Res<SelectionColor>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<&mut Handle<StandardMaterial>>
)
{
    // If a selection event has occured, then do stuff to highlight the selected mesh
    if let Some(event) = selection_reader.reader.latest(&selection_events)
    {
        println!("Found selection event: {:?}", event);

        // If there's something that was previously selected, then we need to set it's material
        // back to it's original material.
        match selection_state.last_selected
        {
            Some(entity) => {
                match query.get_mut::<Handle<StandardMaterial>>(entity)
                {
                    Ok(handle) => {
                        materials.get_mut(&handle).unwrap().albedo = local_last_selected_color.0;
                    },
                    Err(error) => println!("Highlight selected entity query error: {:?}", error)
                }
            },
            None => {}
        }
        
        // Now set the selected entity's material to the selected material.
        match query.get_mut::<Handle<StandardMaterial>>(event.selected)
        {
            Ok(handle) => {
                let color = &mut materials.get_mut(&handle).unwrap().albedo;

                local_last_selected_color.0 = *color;
                *color = selection_color.0;
            },
            Err(error) => println!("Highlight selected entity query error: {:?}", error)
        }
    }
}