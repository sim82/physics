use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::csg;
use crate::editor::components;

use super::components::BrushMaterialProperties;
use super::edit_commands;

// #[derive(Clone)]
pub enum UndoEntry {
    BrushDrag {
        entity: Entity,
        start_brush: csg::Brush,
        brush: csg::Brush,
    },
    EntityAdd {
        entity: Entity,
    },
    MaterialSet {
        entity: Entity,
        face: i32,
        old_material: String,
        material: String,
    },
    BrushRemove {
        entity: Entity,
        brush: csg::Brush,
        material_props: BrushMaterialProperties,
    },
    PointDrag {
        entity: Entity,
        start_transform: Transform,
        transform: Transform,
    },
    Generic {
        cmd: Box<dyn edit_commands::UndoCommand + Send + Sync + 'static>,
    },
}

#[derive(Resource, Default)]
pub struct UndoStack {
    pub stack: Vec<UndoEntry>,
    pub open: bool,
    entity_recreate_map: HashMap<Entity, Entity>,
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

    pub fn push_brush_drag(
        &mut self,
        entity: Entity,
        start_brush: &csg::Brush,
        brush: &csg::Brush,
    ) {
        match (self.open, self.stack.last_mut()) {
            (
                true,
                Some(UndoEntry::BrushDrag {
                    entity: old_entity,
                    start_brush: _,
                    brush: update,
                }),
            ) if entity == *old_entity => {
                // info!("update undo entry");
                *update = brush.clone()
            }
            _ => {
                info!("new undo entry");

                self.stack.push(UndoEntry::BrushDrag {
                    entity,
                    start_brush: start_brush.clone(),
                    brush: brush.clone(),
                });
                self.open = true;
            }
        }

        // info!("undo: {:?}", self.stack);
    }

    pub fn push_entity_add(&mut self, entity: Entity) {
        self.stack.push(UndoEntry::EntityAdd { entity })
    }
    pub fn push_matrial_set(
        &mut self,
        entity: Entity,
        face: i32,
        old_material: String,
        material: String,
    ) {
        self.stack.push(UndoEntry::MaterialSet {
            entity,
            face,
            old_material,
            material,
        })
    }

    pub fn push_brush_remove(
        &mut self,
        entity: Entity,
        brush: csg::Brush,
        material_props: components::BrushMaterialProperties,
    ) {
        self.stack.push(UndoEntry::BrushRemove {
            entity,
            brush,
            material_props,
        });
    }

    pub fn push_point_drag(
        &mut self,
        entity: Entity,
        start_transform: Transform,
        transform: Transform,
    ) {
        match (self.open, self.stack.last_mut()) {
            (
                true,
                Some(UndoEntry::PointDrag {
                    entity: old_entity,
                    start_transform: _,
                    transform: transform_update,
                }),
            ) if entity == *old_entity => {
                // info!("update undo entry");
                *transform_update = transform;
            }
            _ => {
                // info!("new undo entry");

                self.stack.push(UndoEntry::PointDrag {
                    entity,
                    start_transform,
                    transform,
                });
                self.open = true;
            }
        }
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
            Some(UndoEntry::BrushDrag {
                entity,
                start_brush,
                brush: _,
            }) => {
                // let entity = undo_commands.undo_stack.remap_entity(entity);
                // undo_commands
                //     .commands
                //     .entity(entity)
                //     .insert(components::EditUpdate::BrushDrag {
                //         brush: start_brush,
                //         // csg_reprensentation:
                //         //     components::CsgRepresentation {
                //         //         center,
                //         //         radius,
                //         //         csg,
                //         //     },
                //     });
                panic!("outdated");
            }
            Some(UndoEntry::EntityAdd { entity }) => {
                // let entity = undo_commands.undo_stack.remap_entity(entity);
                // undo_commands
                //     .commands
                //     .entity(entity)
                //     .insert(components::Despawn);
                // TODO: remove all entries in entity_recreate_map that point to [entity]
                panic!("outdated");
            }
            Some(UndoEntry::MaterialSet {
                entity,
                face,
                old_material,
                material,
            }) => {
                let entity = undo_commands.undo_stack.remap_entity(entity);

                if let Ok(mut material_props) =
                    undo_commands.material_properties_query.get_mut(entity)
                {
                    material_props.materials[face as usize] = old_material;
                    undo_commands
                        .commands
                        .entity(entity)
                        .insert(components::CsgDirty);
                }
            }
            Some(UndoEntry::BrushRemove {
                entity,
                brush,
                material_props,
            }) => {
                let new_entity = undo_commands
                    .commands
                    .spawn(
                        components::EditorObjectBrushBundle::from_brush(brush)
                            .with_material_properties(material_props),
                    )
                    .id();
                undo_commands
                    .undo_stack
                    .entity_recreate_map
                    .insert(entity, new_entity);
            }
            Some(UndoEntry::PointDrag {
                entity,
                start_transform,
                transform: _,
            }) => {
                if let Ok(mut transform) = undo_commands.transform_query.get_mut(entity) {
                    *transform = start_transform;
                }
            }
            Some(UndoEntry::Generic { cmd }) => cmd.undo(&mut undo_commands),
            None => info!("nothing to undo"),
        }
    }
}
