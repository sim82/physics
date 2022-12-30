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
    fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
        if let Ok((mut material_props, _)) = commands.brush_query.get_mut(self.entity) {
            let old_material = std::mem::replace(
                &mut material_props.materials[self.face as usize],
                self.material,
            );
            commands
                .commands
                .entity(self.entity)
                .insert(components::CsgDirty);

            return Box::new(Undo {
                entity: self.entity,
                face: self.face,
                old_material,
            });
        }

        panic!("material props not found for {:?}", self.entity);
    }
}

impl UndoCommand for Undo {
    fn try_merge(&mut self, other: &dyn UndoCommand) -> bool {
        false
    }

    fn undo(&self, undo_commands: &mut UndoCommands) {
        let entity = undo_commands.undo_stack.remap_entity(self.entity);

        if let Ok(mut material_props) = undo_commands.material_properties_query.get_mut(entity) {
            material_props.materials[self.face as usize] = self.old_material.clone();
            undo_commands
                .commands
                .entity(entity)
                .insert(components::CsgDirty);
        }
    }
}
