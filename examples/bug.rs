use bevy::prelude::*;

#[derive(Component)]
struct A {
    pub i: i32,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct B;

#[derive(Resource)]
struct State {
    timer: Timer,
    c: i32,
}

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(test_system)
        .add_system(change_system)
        .insert_resource(State {
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            c: 0,
        });

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(A { i: 1 });
    commands.spawn(A { i: 2 });
}

fn test_system(
    time: Res<Time>,
    mut state: ResMut<State>,
    mut commands: Commands,
    mut query2: Query<&A, Without<B>>,
    mut query: Query<(Entity, &mut A), With<B>>,
) {
    state.timer.tick(time.delta());

    if !state.timer.just_finished() {
        return;
    }

    if state.c == 0 {
        commands.spawn(A { i: 3 }).insert(B);
        state.c += 1;
    } else if state.c == 1 {
        for (entity, mut a) in &mut query {
            a.i = 7;
            info!("change {:?}", entity);
        }
        state.c += 1;
    }
}

fn change_system(query2: Query<&A>, query: Query<(Entity, &A), Changed<A>>) {
    for (entity, a) in &query {
        info!("changed: {:?} {}", entity, a.i);
    }
}
