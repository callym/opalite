use std::{ cmp::PartialEq, collections::HashMap, ops::{ Deref, DerefMut } };
use conrod::{ self, render::OwnedPrimitives, widget::{ Id, Widget }, Ui };
use gluon;
use specs::{ Dispatcher, World };
use winit::{ EventsLoop, Window };
use crate::{
    Config,
    InputEvent,
    InputEventHandler,
};
use crate::gluon_api::conrod::GluonWidget;

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

pub struct Gluon {
    pub thread: gluon::RootedThread,
    pub compiler: gluon::Compiler,
}

pub struct GluonUi(pub HashMap<String, GluonWidget>);

impl Deref for GluonUi {
    type Target = HashMap<String, GluonWidget>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GluonUi {
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
        let mut name_to_ui = HashMap::new();

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
                let mut gluon_ui = world.write_resource::<GluonUi>();
                let mut generator = ui.widget_id_generator();

                let widgets: Vec<_> = gluon_ui.drain()
                    .map(|(name, widget)| {
                        let id = name_to_ui.entry(name.clone()).or_insert_with(|| generator.next());
                        (widget, *id)
                    })
                    .collect();

                let ui = &mut ui.set_widgets();

                for (widget, id) in widgets.into_iter() {
                    match widget {
                        GluonWidget::BorderedRectangle { value } => value.0.set(id, ui),
                        GluonWidget::Rectangle { value } => value.0.set(id, ui),
                        GluonWidget::Oval { value } => value.0.set(id, ui),
                    };
                }
            }

            {
                let mut opal_ui = world.write_resource::<OpalUi>();
                *opal_ui = OpalUi(ui.draw_if_changed().map(|p| p.owned()));
            }

            dispatcher.dispatch(&mut world.res);
        }

        Ok(())
    }
}
