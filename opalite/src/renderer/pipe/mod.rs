use crate::renderer::{ Buffer, BufferData, PushConstant };

use back::Backend as B;

mod main_pipe;
pub use self::main_pipe::{ MainPipe, Locals as MainLocals, ModelLocals as MainModelLocals };

mod ui_pipe;
pub use self::ui_pipe::{ UiPipe, Locals as UiLocals, ModelLocals as UiModelLocals };

pub struct PipeKey(pub String);

pub trait Pipe {
    type Locals: BufferData;
    type Models;
    type ModelsLocals: PushConstant;

    fn key(&self) -> PipeKey;

    fn locals(&self) -> &Buffer<Self::Locals, B>;

    fn locals_mut(&mut self) -> &mut Buffer<Self::Locals, B>;

    fn update_locals(&mut self, locals: Self::Locals) {
        let locals_buffer = self.locals_mut();
        locals_buffer.write(&[locals]).unwrap();
    }
}
