use nalgebra as na;
use crate::objects::Projectile;

pub fn atmospheric_density(_altitude: f32) -> f32 {
    // Simplified model: constant density
    return 1.225; // kg/m^3 at sea level
}

pub fn force_gravity() -> na::Vector3<f32> {
    return na::Vector3::new(0.0, -9.81, 0.0)
}

pub fn drag_force(projectile: &Projectile) -> na::Vector3<f32> {

    let velocity = projectile.velocity;
    let speed = velocity.norm();
    let _altitude = projectile.position.y;
    let density = atmospheric_density(_altitude);
    let drag_coefficient = projectile.drag_coefficient;
    let area = projectile.effective_area;
    return -0.5 * density * speed * drag_coefficient * area * velocity;
}