use anyhow::Context;

use super::prelude::*;

pub struct Command {
    pub template_entity: Entity,
}

pub use super::add_entity::Undo;

impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Result<Box<dyn UndoCommand + Send + Sync>> {
        let (material_properties, brush) = commands
            .brush_query
            .get(self.template_entity)
            .context("could not find template brush entity.")?;

        let entity = commands
            .commands
            .spawn((
                components::EditorObjectBrushBundle::from_brush(brush.clone())
                    .with_material_properties(material_properties.clone()),
                components::Selected,
            ))
            .id();

        Ok(Box::new(Undo { entity }))

        // panic!("could not find template brush {:?}", self.template_entity);
    }
}
