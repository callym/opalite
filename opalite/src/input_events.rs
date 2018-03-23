use std::collections::HashMap;
use winit::{ ElementState, MouseButton };
use crate::{ Message, MessageSender };

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum InputEvent {
    MouseClicked { state: ElementState, button: MouseButton },
    MouseClickedWithCoordinates {
        state: ElementState,
        button: MouseButton,
        x: f64,
        y: f64,
    },
    MouseCoordinates { x: f64, y: f64 },
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum InputEventType {
    MouseClicked,
    MouseClickedWithCoordinates,
    MouseCoordinates,
}

impl From<InputEvent> for InputEventType {
    fn from(event: InputEvent) -> InputEventType {
        match event {
            InputEvent::MouseClicked { .. } => InputEventType::MouseClicked,
            InputEvent::MouseClickedWithCoordinates { .. } => InputEventType::MouseClickedWithCoordinates,
            InputEvent::MouseCoordinates { .. } => InputEventType::MouseCoordinates,
        }
    }
}

impl Message for InputEvent { }

struct MouseState {
    x: f64,
    y: f64,
}

pub struct InputEventHandler {
    handlers: HashMap<InputEventType, Vec<MessageSender<InputEvent>>>,
    mouse_state: MouseState,
}

impl InputEventHandler {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            mouse_state: MouseState { x: 0.0, y: 0.0 },
        }
    }

    pub fn register(&mut self, ty: InputEventType, sender: MessageSender<InputEvent>) {
        let mut handlers = self.handlers.entry(ty)
            .or_insert(vec![]);
        handlers.push(sender);
    }

    pub fn send(&mut self, event: InputEvent) {
        match event {
            InputEvent::MouseClicked { state, button } => self.send(InputEvent::MouseClickedWithCoordinates {
                state, button,
                x: self.mouse_state.x,
                y: self.mouse_state.y,
            }),
            InputEvent::MouseCoordinates { x, y } => self.mouse_state = MouseState { x, y },
            _ => (),
        };

        if let Some(handlers) = self.handlers.get_mut(&event.into()) {
            for handler in handlers {
                handler.send(event);
            }
        }
    }
}
