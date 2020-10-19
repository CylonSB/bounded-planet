use std::{collections::HashMap, sync::Arc};

use bevy::prelude::*;

pub struct EguiUi {
    pub ui: Option<egui::Ui>,
}

impl FromResources for EguiUi {
    fn from_resources(resources: &Resources) -> Self {
        Self {
            ui: None
        }
    }
}

pub(crate) struct EguiFrameStartEvent {
    pub(crate) new_ui: egui::Ui,
}

// TODO: extend the concept to incorporate a map of arbitrary names to egui's, removing the current singleton restriction
pub(crate) fn egui_state_update(
    mut frame_start_events: ResMut<Events<EguiFrameStartEvent>>,
    mut egui: ResMut<EguiUi>,
) {
    // Grab the newest ui provided by an event
    let new_ui = frame_start_events.drain().next();

    let EguiFrameStartEvent { new_ui } = if egui.ui.is_none() && new_ui.is_none() {
        println!("Didn't find a ui this time...");
        return;
    } else {
        new_ui.expect("Didn't recieve a Ui to use for next frame! Without a Ui, nothing works...")
    };

    // Replace the old ui with the new one from the event
    egui.ui = Some(new_ui);
}

pub struct EguiContextEntityMap {
    contexts: HashMap<usize, Entity>,
}

impl EguiContextEntityMap {
    pub fn get_entity(&self, context: &Arc<egui::Context>) -> Option<Entity> {
        let ptr_value = Arc::as_ptr(context) as usize;
        self.contexts.get(&ptr_value).copied()
    }

    pub fn insert_entity(&mut self, context: &Arc<egui::Context>, entity: Entity) {
        let ptr_value = Arc::as_ptr(context) as usize;
        self.contexts.insert(ptr_value, entity);
    }
}