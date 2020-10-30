use std::sync::Arc;

use bevy::{
    prelude::*,
    input::{
        keyboard::KeyboardInput,
        mouse::{
            MouseButtonInput,
            MouseWheel
        }
    },
    render::{
        draw::DrawContext,
        render_graph::RenderGraph,
        renderer::RenderResourceBindings
    },
    window::{
        CursorMoved,
        WindowId,
        WindowResized
    }
};

use egui::Event;

use crate::{
    render::AddEguiSystemNode,
    components::EguiJobsDescriptor,
    egui_node::EguiNode,
    egui_node::{
        EguiSystemNode,
    },
};

#[allow(clippy::type_complexity)]
pub fn egui_draw_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    msaa: Res<Msaa>,
    mut query: Query<With<Handle<EguiNode>, (&mut Draw, &mut RenderPipelines, &EguiJobsDescriptor)>>,
) {
    for (mut draw, mut render_pipelines, jobs_descriptor) in &mut query.iter() {
        if !draw.is_visible {
            continue;
        }

        let render_pipelines = &mut *render_pipelines;
        for pipeline in render_pipelines.pipelines.iter_mut() {
            pipeline.specialization.sample_count = msaa.samples;
        }

        // This is needed since draw operations were already done for entity (and it cannot currently be prevented). In fact...
        // TODO(#55): stop the entity from having its draw operations already done, allowing this to be removed
        draw.clear_render_commands();

        for render_pipeline in render_pipelines.pipelines.iter() {
            draw_context
                .set_pipeline(
                    &mut draw,
                    render_pipeline.pipeline,
                    &render_pipeline.specialization,
                )
                .unwrap();
            draw_context
                .set_bind_groups_from_bindings(
                    &mut draw,
                    &mut [
                        &mut render_pipelines.bindings,
                        &mut render_resource_bindings,
                    ],
                )
                .unwrap();
            let indices = draw_context
                .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
                .unwrap();


            if indices.is_some() {
                for (indices, base_vertex) in &jobs_descriptor.jobs {
                    draw.draw_indexed(indices.clone(), *base_vertex, 0..1);
                }
            }
        }
    }
}

// TODO(#56): properly integrate the context as an asset to remove the singleton restriction on egui contexts
pub struct EguiContext {
    // TODO(#56): Utilize name as a key for properly integrated context asset
    // name: &'static str,
    pub context: Arc<egui::Context>,
}

impl Default for EguiContext {
    fn default() -> Self {
        Self {
            context: egui::Context::new(),
        }
    }
}

#[derive(Default)]
pub struct EguiSystemNodeAdderState {
    event_reader: EventReader<AssetEvent<EguiContext>>,
}

// TODO(#56): properly integrate the context as an asset to remove the singleton restriction on egui contexts
pub fn egui_system_node_adder(
    mut state: Local<EguiSystemNodeAdderState>,
    egui_input: Res<EguiInput>,
    
    context_events: Res<Events<AssetEvent<EguiContext>>>,
    mut contexts: ResMut<Assets<EguiContext>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    for event in state.event_reader.iter(&context_events) {
        match event {
            AssetEvent::Created { handle } => {
                let EguiContext { context, .. } = contexts.get_mut(handle).unwrap();

                // Begin frame so that the system is in the correct state
                context.begin_frame(egui_input.raw_input.clone());

                render_graph.add_egui_system_node(EguiSystemNode {
                    command_queue: Default::default(),
                    context: *handle,
                });
            },
            AssetEvent::Modified { .. } => {
                // TODO(#56): Determine what (if anything?) should happen if there is a modification
            },
            AssetEvent::Removed { .. } => {
                todo!("TODO(#56): Deal with removing the render node stuff for a given egui context!")
            }
        }
    }
}

pub struct GatherEguiInputState {
    mouse_button_reader: EventReader<MouseButtonInput>,
    cursor_moved_reader: EventReader<CursorMoved>,
    mouse_wheel_reader: EventReader<MouseWheel>,
    keyboard_input_reader: EventReader<KeyboardInput>,

    window_resized_reader: EventReader<WindowResized>,
    primary_window_id: WindowId,

    current_tick: f64,
    previous_input: egui::RawInput,
}

impl FromResources for GatherEguiInputState {
    fn from_resources(resources: &Resources) -> Self {
        let windows = resources.get::<Windows>().unwrap();
        let window = windows.get_primary().unwrap();

        let screen_size = egui::vec2(window.width as _, window.height as _);

        Self {
            mouse_button_reader: Default::default(),
            cursor_moved_reader: Default::default(),
            mouse_wheel_reader: Default::default(),
            keyboard_input_reader: Default::default(),

            window_resized_reader: Default::default(),
            primary_window_id: window.id,

            current_tick: 0.0,
            previous_input: egui::RawInput {
                mouse_down: false,
                mouse_pos: None,
                scroll_delta: egui::Vec2::new(0.0, 0.0),
                screen_size,
                pixels_per_point: Some(1.0),
                time: 0.0,
                events: vec![]
            }
        }
    }
}

/// Stores the current frame's [`RawInput`] to be passed to egui.
pub struct EguiInput {
    pub raw_input: egui::RawInput
}

impl Default for EguiInput {
    fn default() -> Self {
        Self {
            raw_input: egui::RawInput {
                mouse_down: false,
                mouse_pos: None,
                scroll_delta: egui::math::vec2(0.0, 0.0),
                screen_size: egui::math::vec2(1280.0, 720.0),
                pixels_per_point: Some(1.0),
                time: 0.0,
                events: Vec::new()
            }
        }
    }
}

/// Gathers all inputs to update [`EguiInput`] so egui can start frames.
pub fn egui_gather_input(
    mut state: Local<GatherEguiInputState>,
    mut egui_input: ResMut<EguiInput>,
    
    mouse_button_events: Res<Events<MouseButtonInput>>,
    cursor_moved_events: Res<Events<CursorMoved>>,
    mouse_wheel_events: Res<Events<MouseWheel>>,
    keyboard_input_events: Res<Events<KeyboardInput>>,

    window_resized_events: Res<Events<WindowResized>>,
) {
    state.current_tick += 1.0;

    // Easier to allow the warning then rename the fields and break the shorthand syntax
    #[allow(unused_variables)]
    let GatherEguiInputState {
        mouse_button_reader,
        cursor_moved_reader,
        mouse_wheel_reader,
        keyboard_input_reader,

        window_resized_reader,
        primary_window_id,
        
        current_tick,
        previous_input
    } = &mut *state;

    // Uses the pressed state of the latest left mouse event, otherwise the previous state
    let mouse_down = mouse_button_reader
        .find_latest(&mouse_button_events, |input| {
            input.button == MouseButton::Left
        })
        .map_or(previous_input.mouse_down, |input| {
            input.state.is_pressed()
        });

    let scroll_delta = mouse_wheel_reader.iter(&mouse_wheel_events)
        .fold(egui::vec2(0.0, 0.0), |delta, MouseWheel { x, y, .. }| {
            delta + egui::vec2(*x, *y)
        });

    let screen_size = window_resized_reader.find_latest(&window_resized_events, |resized| {
            resized.id == *primary_window_id
        })
        .map_or(previous_input.screen_size, |WindowResized { width, height, .. }| {
            egui::vec2(*width as _, *height as _)
        });

    let mouse_pos = cursor_moved_reader
        .find_latest(&cursor_moved_events, |cursor| {
            cursor.id == *primary_window_id
        })
        .map_or(previous_input.mouse_pos, |CursorMoved { position, .. }| {
            // Bevy has origin as bottom left, egui expects origin to be top left, so we do the math to change it
            Some(egui::pos2(position.x(), screen_size.y - position.y()))
        });

    let events = keyboard_input_reader.iter(&keyboard_input_events)
        .fold(Vec::<egui::Event>::new(), |mut events: Vec<egui::Event>, KeyboardInput { key_code, state, .. }| {
            if let Some(key_code) = key_code {
                // Handle special cased key combos like cut/copy/paste
                if let Some(event) = match key_code {
                    KeyCode::Copy => Some(Event::Copy),
                    KeyCode::Cut => Some(Event::Cut),
                    KeyCode::Paste => {
                        tracing::warn!("Paste event isn't implemented in bevy_egui! We need to figure out how to get stuff from the clipboard...");
                        None
                    }
                    _ => None
                } {
                    events.push(event);
                } else if let Some(key) = key_code.into_egui_key() {
                    events.push(Event::Key {
                        key,
                        pressed: state.is_pressed()
                    });
                } else {
                    tracing::info!("Key presses are currently not really given to egui! Pressed: {:?}", key_code);
                }
            }

            events
        });

    let new_input = egui::RawInput {
        mouse_down,
        mouse_pos,
        scroll_delta,
        screen_size,
        events,
        ..*previous_input
    };

    *previous_input = new_input.clone();
    egui_input.raw_input = new_input;
}

trait IntoEguiKey {
    fn into_egui_key(self) -> Option<egui::Key>;
}

impl IntoEguiKey for KeyCode {
    fn into_egui_key(self) -> Option<egui::Key> {
        let key = match self {
            KeyCode::LAlt |
                KeyCode::RAlt =>    egui::Key::Alt,
            KeyCode::Back =>        egui::Key::Backspace,
            KeyCode::LControl |
                KeyCode::RControl =>egui::Key::Control,
            KeyCode::Delete =>      egui::Key::Delete,
            KeyCode::Down =>        egui::Key::Down,
            KeyCode::End =>         egui::Key::End,
            KeyCode::Escape =>      egui::Key::Escape,
            KeyCode::Home =>        egui::Key::Home,
            KeyCode::Insert =>      egui::Key::Insert,
            KeyCode::Left =>        egui::Key::Left,
            KeyCode::LWin |
                KeyCode::RWin =>    egui::Key::Logo,
            KeyCode::PageDown =>    egui::Key::PageDown,
            KeyCode::PageUp =>      egui::Key::PageUp,
            KeyCode::NumpadEnter |
                KeyCode:: Return => egui::Key::Enter,
            KeyCode::Right =>       egui::Key::Right,
            KeyCode::LShift |
                KeyCode::RShift =>  egui::Key::Shift,
            KeyCode::Tab =>         egui::Key::Tab,
            KeyCode::Up =>          egui::Key::Up,
            
            _ => return None
        };

        Some(key)
    }
}