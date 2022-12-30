use super::prelude::*;

pub struct Command;
pub use super::add_entity::Undo;
impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
        let entity = commands
            .commands
            .spawn((
                components::EditorObjectPointlightBundle::default(),
                components::Selected,
            ))
            .id();

        Box::new(Undo { entity })
    }
}
