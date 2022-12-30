use super::prelude::*;

pub struct Command {
    pub template_entity: Entity,
}

pub use super::add_entity::Undo;

impl EditCommand for Command {
    fn apply(self, commands: &mut EditCommands) -> Box<dyn UndoCommand + Send + Sync> {
        if let Ok((material_properties, brush)) = commands.brush_query.get(self.template_entity) {
            let entity = commands
                .commands
                .spawn((
                    components::EditorObjectBrushBundle::from_brush(brush.clone())
                        .with_material_properties(material_properties.clone()),
                    components::Selected,
                ))
                .id();

            return Box::new(Undo { entity });
        }

        // FIXME: make apply fallible (if that makes sense)
        panic!("could not find template brush {:?}", self.template_entity);
    }
}
