use crate::trace::CollisionSystem;

use bevy::{math::Vec3, prelude::*};

// port of quake 3 PM_ClipVelocity
fn do_clip_velocity(v_in: Vec3, normal: Vec3, overbounce: f32) -> Vec3 {
    let backoff = match v_in.dot(normal) {
        dot if dot < 0.0 => dot * overbounce,
        dot => dot / overbounce,
    };
    let change = Vec3::new(normal.x * backoff, normal.y * backoff, normal.z * backoff);
    v_in - change
}

pub fn slidemove_try2(
    collision_system: &CollisionSystem,
    // contact_debug: &mut ContactDebug,
    // debug_lines: &mut debug_lines::DebugLines,
    // collider_query: &QueryPipelineColliderComponentsQuery,
    origin: Vec3,
    mut velocity: Vec3,
    mut time: f32,
    // query_pipeline: &Res<QueryPipeline>,
) -> (Vec3, Vec3, bool) {
    let mut planes = Vec::new();
    let mut move_v = Vec3::ZERO;

    let gravity = !true;
    let gravity_normal = -Vec3::Y;
    let gravity_vector = gravity_normal * 9.81;

    let end_velocity = if gravity {
        let end_velocity = velocity + gravity_vector * time;
        velocity = (velocity + end_velocity) * 0.5;
        end_velocity
    // primal_velocity = endVelocity;
    // if ( groundPlane ) {
    // 	// slide along the ground plane
    // 	current.velocity.ProjectOntoPlane( groundTrace.c.normal, OVERCLIP );
    // }
    } else {
        velocity
    };

    info!("slidemove {:?} {:?} {}", origin, velocity, time);
    // initial velocity defines first clipping plane -> avoid to be nudged backwards (due to overclip?)
    planes.push(velocity.normalize());
    planes.push(Vec3::Y);

    'bump: for bump in 0..4 {
        // check of end pos can be reached without collision
        let trace_start_pos = origin + move_v;
        let trace_dist = velocity * time;

        let mut res = collision_system.trace2(trace_start_pos, trace_dist);

        time -= time * res.f;
        move_v += res.dist;

        if res.f >= 1.0 {
            break;
        }

        let mut stepped = false;
        let can_step = true;

        if can_step {
            // todo: trace to ground
            let near_ground = true;

            if near_ground {
                const MAX_STEP_HEIGHT: f32 = 0.12;

                let mut step_v = move_v;

                // step up
                let trace_start_pos = origin + step_v;
                let trace_dist = -gravity_normal * MAX_STEP_HEIGHT;
                let up_res = collision_system.trace2(trace_start_pos, trace_dist);
                step_v += up_res.dist;

                // step along velocity
                let trace_start_pos = origin + step_v;
                let trace_dist = velocity * time;

                let step_res = collision_system.trace2(trace_start_pos, trace_dist);
                step_v += step_res.dist;

                // step down
                let trace_start_pos = origin + step_v;
                let trace_dist = gravity_normal * MAX_STEP_HEIGHT;
                let down_res = collision_system.trace2(trace_start_pos, trace_dist);
                step_v += down_res.dist;

                if step_res.f >= 1.0 {
                    time = 0.0;
                    move_v = step_v;
                    break;
                }

                if step_res.f > res.f {
                    time -= time * step_res.f;
                    move_v = step_v;
                    stepped = true;
                    res = step_res;
                }
            }
        }
        //
        // if this is the same plane we hit before, nudge velocity
        // out along it, which fixes some epsilon issues with
        // non-axial planes
        //
        if let Some(contact) = res.contact {
            for plane in planes.iter() {
                if contact.collider_normal.dot(*plane) > 0.999 {
                    velocity += contact.collider_normal;
                    continue 'bump;
                }
            }
            planes.push(contact.collider_normal);
        }

        for (i, plane_i) in planes.iter().cloned().enumerate() {
            let into = velocity.dot(plane_i);
            if into >= 0.1 {
                continue;
            }
            const OVERCLIP: f32 = 1.001;
            let mut clip_velocity = do_clip_velocity(velocity, plane_i, OVERCLIP);

            for (j, plane_j) in planes.iter().cloned().enumerate() {
                if j == i {
                    continue;
                }
                if clip_velocity.dot(plane_j) >= 0.1 {
                    continue;
                }
                clip_velocity = do_clip_velocity(clip_velocity, plane_j, OVERCLIP);

                if clip_velocity.dot(plane_i) >= 0.0 {
                    continue;
                }
                let dir = plane_i.cross(plane_j).normalize();
                let d = dir * velocity;
                clip_velocity = d * dir;

                for (k, plane_k) in planes.iter().cloned().enumerate() {
                    if k == i || k == j {
                        continue;
                    }
                    if clip_velocity.dot(plane_k) >= 0.1 {
                        continue;
                    }
                    velocity = Vec3::ZERO;
                    return (Vec3::ZERO, end_velocity, true);
                }
            }
            velocity = clip_velocity;
            break;
        }
    }
    (move_v, end_velocity, true)
    // todo!()
}
