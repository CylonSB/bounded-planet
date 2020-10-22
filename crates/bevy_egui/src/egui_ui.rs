use bevy::prelude::*;

pub struct EguiUi {
    pub ui: Option<egui::Ui>,
}

impl FromResources for EguiUi {
    fn from_resources(_resources: &Resources) -> Self {
        Self {
            ui: None,
        }
    }
}

pub(crate) struct EguiFrameStartEvent {
    pub(crate) new_ui: egui::Ui,
}

// TODO(#56): move egui context updates to the start of the frame along with utilizing a context map, to remove the singleton restriction
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