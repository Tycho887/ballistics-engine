use nalgebra as na;
use crate::forces::{force_gravity, drag_force};
use crate::objects::Projectile;

pub mod forces;
pub mod objects;

fn update_projectile(projectile: &mut Projectile, time_step: f32) {
    //Update the projectile's position and velocity based on forces acting on it.
    let gravity = force_gravity() * projectile.mass;
    let drag = drag_force(&projectile);
    let total_force = gravity + drag;

    let acceleration = total_force / projectile.mass; // Assuming mass = 1 for simplicity
    projectile.velocity += acceleration * time_step;
    projectile.position += projectile.velocity * time_step;
}

fn main() {
    let mut ballistic_rocket = Projectile {
        mass: 100f32,
        effective_area: 1f32,
        position: na::Vector3::new(0.0f32, 0.0f32, 0.0f32),
        velocity: na::Vector3::new(100.0f32, 100.0f32, 0.0f32),
        drag_coefficient: 0.1f32,
    };

    let time_step: f32 = 0.1f32;

    let mut seconds: f32 = 0f32;
    while ballistic_rocket.position.y >= 0.0 {
        update_projectile(&mut ballistic_rocket, time_step);
        seconds += time_step;
        println!("Time: {}s, Position: {:?}, Velocity: {:?}", seconds, ballistic_rocket.position, ballistic_rocket.velocity);
    }
    println!("Projectile has hit the ground at time: {}s", seconds);

}
