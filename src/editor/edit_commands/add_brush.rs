use super::prelude::*;

pub struct Command {
    pub brush: csg::Brush,
}
pub use super::add_entity::Undo;

impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Result<Box<dyn UndoCommand + Send + Sync>> {
        let entity = commands
            .commands
            .spawn((
                components::EditorObjectBrushBundle::from_brush(self.brush),
                components::Selected,
            ))
            .id();

        Ok(Box::new(Undo { entity }))
    }
}
