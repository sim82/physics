use super::prelude::*;

pub struct Command {
    pub entity: Entity,
    pub face: i32,
    pub material: String,
}

pub struct Undo {
    pub entity: Entity,
    pub face: i32,
    pub old_material: String,
}

impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Result<Box<dyn UndoCommand + Send + Sync>> {
        // fallible stuff
        let (mut material_props, _) = commands.brush_query.get_mut(self.entity)?;
        commands
            .commands
            .get_entity(self.entity)
            .ok_or(EditCommandError::UnknownEntity(self.entity))?
            .insert(components::CsgDirty);
        // point of no return

        let old_material = std::mem::replace(
            &mut material_props.materials[self.face as usize],
            self.material,
        );

        Ok(Box::new(Undo {
            entity: self.entity,
            face: self.face,
            old_material,
        }))

        // panic!("material props not found for {:?}", self.entity);
    }
}

impl UndoCommand for Undo {
    fn try_merge(&mut self, _other: &dyn UndoCommand) -> bool {
        false
    }

    fn undo(&self, undo_commands: &mut UndoCommands) -> Result<()> {
        let entity = undo_commands.undo_stack.remap_entity(self.entity);
        // fallible stuff
        let mut material_props = undo_commands.material_properties_query.get_mut(entity)?;
        undo_commands
            .commands
            .get_entity(entity)
            .ok_or(EditCommandError::UnknownEntity(entity))?
            .insert(components::CsgDirty);

        // point of no return

        material_props.materials[self.face as usize] = self.old_material.clone();
        Ok(())
    }
}
