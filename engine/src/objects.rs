use nalgebra as na;

pub struct Projectile {
    pub mass: f32,
    pub effective_area: f32,
    pub position: na::Vector3<f32>,
    pub velocity: na::Vector3<f32>,
    pub drag_coefficient: f32,
}