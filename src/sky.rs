use bevy::prelude::*;
use bevy_atmosphere::prelude::*;

// Marker for updating the position of the light, not needed unless we have multiple lights
#[derive(Component)]
struct Sun;

// Timer for updating the daylight cycle (updating the atmosphere every frame is slow, so it's better to do incremental changes)
#[derive(Resource)]
struct CycleTimer(Timer, usize);

#[allow(unused)]
// We can edit the Atmosphere resource and it will be updated automatically
fn daylight_cycle(
    mut atmosphere: ResMut<Atmosphere>,
    mut query: Query<(&mut Transform, &mut DirectionalLight), With<Sun>>,
    mut timer: ResMut<CycleTimer>,
    time: Res<Time>,
) {
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let t = time.elapsed_seconds_wrapped() as f32 / 2.0;
        atmosphere.sun_position = Vec3::new(0., t.sin(), t.cos());

        if let Some((mut light_trans, mut directional)) = query.single_mut().into() {
            light_trans.rotation = Quat::from_rotation_x(-t.sin().atan2(t.cos()));
            directional.illuminance = t.sin().max(0.0).powf(2.0) * 100000.0;
        }
    }
}

#[allow(unused)]
// Simple environment
fn setup_environment(mut commands: Commands) {
    // Our Sun
    commands.spawn((
        DirectionalLightBundle {
            ..Default::default()
        },
        Sun, // Marks the light as Sun
    ));
}

#[allow(unused)]
fn examples_cycle(mut commands: Commands, time: Res<Time>, mut timer: ResMut<CycleTimer>) {
    timer.0.tick(time.delta());

    if timer.0.finished() {
        let atmospheres = [
            Atmosphere {
                sun_position: Vec3::new(0., 0., -1.),
                ..default()
            },
            Atmosphere {
                sun_position: Vec3::new(0., 0., -1.),
                rayleigh_coefficient: Vec3::new(1e-5, 1e-5, 1e-5),
                ..default()
            },
            Atmosphere {
                rayleigh_coefficient: Vec3::new(2e-5, 1e-5, 2e-5),
                ..default()
            },
            Atmosphere {
                mie_coefficient: 5e-5,
                ..default()
            },
            Atmosphere {
                rayleigh_scale_height: 16e3,
                mie_scale_height: 2.4e3,
                ..default()
            },
            Atmosphere {
                sun_intensity: 11.0,
                ..default()
            },
            Atmosphere {
                ray_origin: Vec3::new(0., 6372e3 / 2., 0.),
                planet_radius: 6371e3 / 2.,
                atmosphere_radius: 6471e3 / 2.,
                ..default()
            },
            Atmosphere {
                ray_origin: Vec3::new(6372e3, 0., 0.),
                ..default()
            },
            Atmosphere {
                mie_direction: -0.758,
                ..default()
            },
        ];
        commands.insert_resource(atmospheres[timer.1 % atmospheres.len()]);
        timer.1 += 1;
    }
}

pub struct SkyPlugin;
impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Atmosphere::default()) // Default Atmosphere material, we can edit it to simulate another planet
            .insert_resource(CycleTimer(
                Timer::new(
                    bevy::utils::Duration::from_millis(5000), // Update our atmosphere every 50ms (in a real game, this would be much slower, but for the sake of an example we use a faster update)
                    TimerMode::Repeating,
                ),
                0,
            ))
            .add_plugin(AtmospherePlugin) // Default AtmospherePlugin
            // .add_startup_system(setup_environment)
            // .add_system(daylight_cycle)
            // .add_system(examples_cycle)
            ;
    }
}
