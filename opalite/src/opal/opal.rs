use std::{ cmp::PartialEq, ops::{ Deref, DerefMut } };
use cgmath::Vector3;
use conrod::{ self, render::OwnedPrimitives, widget::{ id::Generator, Id }, Ui };
use specs::{ DispatcherBuilder, Dispatcher, World };
use winit::{ EventsLoop, WindowBuilder, Window };
use crate::{
    AiComponent,
    AiSystem,
    CollisionLayers,
    Config,
    ConfigBuilder,
    InitialPosition,
    InputEvent,
    InputEventHandler,
    MapMessage,
    MessageSender,
    ModelData,
    ModelKey,
    Map,
    MapSystem,
    Position,
    Renderer,
    RLock,
    Shard,
};

pub struct WindowClosed(pub(super) bool);

impl PartialEq<bool> for WindowClosed {
    fn eq(&self, other: &bool) -> bool {
        &self.0 == other
    }
}

pub struct OpalUi(pub(super) Option<OwnedPrimitives>);

impl Deref for OpalUi {
    type Target = Option<OwnedPrimitives>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OpalUi {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct Opal<'a, 'b> {
    pub(super) config: Config,
    pub(super) dispatcher: Dispatcher<'a, 'b>,
    pub(super) events_loop: EventsLoop,
    pub(super) input_event_handler: InputEventHandler,
    pub(super) ui: Ui,
    #[allow(dead_code)]
    pub(super) window: Window,
    pub(super) world: World,
}

impl<'a, 'b> Opal<'a, 'b> {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn input_event_handler(&self) -> &InputEventHandler {
        &self.input_event_handler
    }

    pub fn input_event_handler_mut(&mut self) -> &mut InputEventHandler {
        &mut self.input_event_handler
    }

    pub fn ui_id(&mut self) -> Id {
        let mut generator = self.ui.widget_id_generator();
        generator.next()
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn run(&mut self) -> Result<(), ()> {
        use winit::{ Event, WindowEvent };

        let Opal { dispatcher, events_loop, input_event_handler, ui, window, world, .. } = self;

        while *world.read_resource::<WindowClosed>() == false {
            events_loop.poll_events(|event| {
                if let Event::WindowEvent { event, .. } = event.clone() {
                    match event {
                        WindowEvent::Closed => {
                            let mut window_closed = world.write_resource::<WindowClosed>();
                            *window_closed = WindowClosed(true);
                        },
                        WindowEvent::CursorMoved { position, .. } => {
                            input_event_handler.send(InputEvent::MouseCoordinates {
                                x: position.0,
                                y: position.1,
                            });
                        },
                        WindowEvent::MouseInput { state, button, .. } => {
                            input_event_handler.send(InputEvent::MouseClicked {
                                state, button
                            });
                        }
                        _ => (),
                    }
                }

                match conrod::backend::winit::convert_event(event.clone(), window) {
                    Some(event) => ui.handle_event(event),
                    None => (),
                };
            });

            {
                let mut opal_ui = world.write_resource::<OpalUi>();
                *opal_ui = OpalUi(ui.draw_if_changed().map(|p| p.owned()));
            }

            dispatcher.dispatch(&mut world.res);
        }

        Ok(())
    }
}
