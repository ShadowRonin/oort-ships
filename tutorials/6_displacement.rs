// Tutorial: Deflection
// Destroy the enemy ship. Its position is given by the "target" function and velocity by the
// "target_velocity" function.
//
// Hint: p = p₀ + v₀t + ½at² (the third equation of kinematics)
//
// p.s. You can change your username by clicking on it at the top of the page.
use oort_api::prelude::*;

const BULLET_SPEED: f64 = 1000.0; // m/s

pub struct Ship {
    target_offset_deg: f64,
    target_offset_deg_increment: f64,

    prev_target_vel: Vec2,
}

fn degree_to_radian(deg: f64) -> f64 {
    deg * (PI / 180.0)
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            target_offset_deg: 5.0,
            target_offset_deg_increment: 1.0,
            prev_target_vel: vec2(0.0,0.0),
            predicted_pf: vec2(0.0,0.0),
        }
    }

    

    fn calc_future_target(&mut self, depth_of_calc: u32) -> Vec2 {
        // Hint: p = p₀ + v₀t + ½at² (the third equation of kinematics)
        let p0 = target();
        let v = target_velocity();
        let mut t = (target() - position()).length() / BULLET_SPEED;
        let a = self.prev_target_vel - target_velocity();
        let mut pf = p0 + (v * t) + (0.5 * a * (t * t));

        // TODO: maybe just have some percent offset for t, instead of recalculating
        for _ in [..depth_of_calc] {
            t = (pf - position()).length() / BULLET_SPEED;
            pf = p0 + v * t + 0.5 * a * t * t;
        }

        self.prev_target_vel = target_velocity();
        pf
    }

    pub fn tick(&mut self) {
        draw_line(position(), target(), 0x00ff00);

        let target_in_time = self.calc_future_target(100);
        let target_in_time_ang = angle_diff(heading(), target_in_time.angle());

        draw_line(position(), target_in_time, 0x47cbe6);

        if target_in_time_ang.abs() < degree_to_radian(0.5) {
            fire(0);
        }
        turn(target_in_time_ang * 100.0);
    }
}
