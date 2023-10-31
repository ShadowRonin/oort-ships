// Tutorial: Lead
// Destroy the enemy ship. Its position is given by the "target" function and velocity by the
// "target_velocity" function. Your ship is not able to accelerate in this scenario.
//
// This is where the game becomes challenging! You'll need to lead the target
// by firing towards where the target will be by the time the bullet gets there.
//
// Hint: target() + target_velocity() * t gives the position of the target after t seconds.
//
// You can scale a vector by a number: vec2(a, b) * c == vec2(a * c, b * c)
//
// p.s. You can change your username by clicking on it at the top of the page.
use oort_api::prelude::*;

const BULLET_SPEED: f64 = 1000.0; // m/s

pub struct Ship {
    target_offset_deg: f64,
    target_offset_deg_increment: f64,
}

fn degree_to_radian(deg: f64) -> f64 {
    deg * (PI / 180.0)
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            target_offset_deg: 5.0,
            target_offset_deg_increment: 1.0,
        }
    }

    

    fn calc_future_target(&mut self, depth_of_calc: u32) -> Vec2 {
        let mut i = 0;
        let mut dp = target() - position();
        let mut time_to_target = dp.length() / BULLET_SPEED;
        let mut target_in_time = target() + (target_velocity() * time_to_target);
        while i < depth_of_calc {
            i = i + 1;
            dp = target_in_time - position();
            time_to_target = dp.length() / BULLET_SPEED;
            target_in_time = target() + (target_velocity() * time_to_target);
        }
        target_in_time
    }

    pub fn tick(&mut self) {
        draw_line(position(), target(), 0x00ff00);
        let dp = target() - position();
        debug!("distance to target: {}", dp.length());
        debug!("time to target: {}", dp.length() / BULLET_SPEED);

        let target_in_time = self.calc_future_target(10);
        let target_in_time_ang = angle_diff(heading(), target_in_time.angle());
        debug!("target in time: {}", target_in_time);

        draw_line(position(), target_in_time, 0x47cbe6);

        if target_in_time_ang.abs() < degree_to_radian(5.0) {
            fire(0);
        }
        turn(target_in_time_ang * 100.0);
    }
}
