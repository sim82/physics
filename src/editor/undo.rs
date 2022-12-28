use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::csg;
use crate::editor::components;

use super::components::BrushMaterialProperties;

#[derive(Debug, Clone)]
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
}

#[derive(Resource, Default)]
pub struct UndoStack {
    pub stack: Vec<UndoEntry>,
    pub open: bool,
    entity_recreate_map: HashMap<Entity, Entity>,
}

impl UndoStack {
    fn remap_entity(&self, entity: Entity) -> Entity {
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

pub fn undo_system(
    mut commands: Commands,
    keycodes: Res<Input<KeyCode>>,
    mut undo_stack: ResMut<UndoStack>,
    mut material_properties_query: Query<&mut components::BrushMaterialProperties>,
    mut transform_query: Query<&mut Transform>,
) {
    if keycodes.just_pressed(KeyCode::Z) {
        let undo_entry = undo_stack.stack.pop();
        info!("undo: {:?}", undo_entry);
        match undo_entry {
            Some(UndoEntry::BrushDrag {
                entity,
                start_brush,
                brush: _,
            }) => {
                let entity = undo_stack.remap_entity(entity);
                commands
                    .entity(entity)
                    .insert(components::EditUpdate::BrushDrag {
                        brush: start_brush,
                        // csg_reprensentation:
                        //     components::CsgRepresentation {
                        //         center,
                        //         radius,
                        //         csg,
                        //     },
                    });
            }
            Some(UndoEntry::EntityAdd { entity }) => {
                let entity = undo_stack.remap_entity(entity);
                commands.entity(entity).insert(components::Despawn);
                // TODO: remove all entries in entity_recreate_map that point to [entity]
            }
            Some(UndoEntry::MaterialSet {
                entity,
                face,
                old_material,
                material,
            }) => {
                let entity = undo_stack.remap_entity(entity);

                if let Ok(mut material_props) = material_properties_query.get_mut(entity) {
                    material_props.materials[face as usize] = old_material;
                }
            }
            Some(UndoEntry::BrushRemove {
                entity,
                brush,
                material_props,
            }) => {
                let new_entity = commands
                    .spawn(
                        components::EditorObjectBrushBundle::from_brush(brush)
                            .with_material_properties(material_props),
                    )
                    .id();
                undo_stack.entity_recreate_map.insert(entity, new_entity);
            }
            Some(UndoEntry::PointDrag {
                entity,
                start_transform,
                transform: _,
            }) => {
                if let Ok(mut transform) = transform_query.get_mut(entity) {
                    *transform = start_transform;
                }
            }
            None => info!("nothing to undo"),
        }
    }
}
