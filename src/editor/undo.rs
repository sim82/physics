use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::csg;
use crate::editor::components;

use super::components::BrushMaterialProperties;
use super::edit_commands;

// #[derive(Clone)]
pub enum UndoEntry {
    Generic {
        cmd: Box<dyn edit_commands::UndoCommand + Send + Sync + 'static>,
    },
}

#[derive(Resource, Default)]
pub struct UndoStack {
    pub stack: Vec<UndoEntry>,
    pub open: bool,
    pub entity_recreate_map: HashMap<Entity, Entity>,
}

impl UndoStack {
    pub fn remap_entity(&self, entity: Entity) -> Entity {
        match self.entity_recreate_map.get(&entity) {
            Some(mapped_entity) => {
                info!("remap {:?} {:?}", entity, mapped_entity);
                *mapped_entity
            }
            None => entity,
        }
    }
    pub fn commit(&mut self) {
        info!("commit");
        self.open = false;
    }
    pub fn push_generic(
        &mut self,
        cmd: Box<dyn edit_commands::UndoCommand + Send + Sync + 'static>,
    ) {
        if let (true, Some(UndoEntry::Generic { cmd: top_cmd })) =
            (self.open, self.stack.last_mut())
        {
            if top_cmd.try_merge(cmd.as_ref()) {
                return;
            }
        }
        self.stack.push(UndoEntry::Generic { cmd });
        self.open = true;
    }
}

#[derive(SystemParam)]
pub struct UndoCommands<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub material_properties_query: Query<'w, 's, &'static mut components::BrushMaterialProperties>,
    pub transform_query: Query<'w, 's, &'static mut Transform>,
    pub undo_stack: ResMut<'w, UndoStack>,
}

pub fn undo_system(mut undo_commands: UndoCommands, keycodes: Res<Input<KeyCode>>) {
    if keycodes.just_pressed(KeyCode::Z) {
        let undo_entry = undo_commands.undo_stack.stack.pop();
        // info!("undo: {:?}", undo_entry);
        match undo_entry {
            Some(UndoEntry::Generic { cmd }) => cmd.undo(&mut undo_commands),
            None => info!("nothing to undo"),
        }
    }
}
