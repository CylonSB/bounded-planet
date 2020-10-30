use bevy::prelude::*;

use tracing::trace;

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
        trace!("Did not find a UI this tick!");
        return;
    } else {
        // This should never occur. If it does, something has broke bevy and the [`EguiSystemNode`] that sends this event.
        new_ui.expect("Previously an egui ui has been received, but one hasn't been received this frame!")
    };

    // Replace the old ui with the new one from the event
    egui.ui = Some(new_ui);
}