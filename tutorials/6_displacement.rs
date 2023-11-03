// Tutorial: Deflection
// Destroy the enemy ship. Its position is given by the "target" function and velocity by the
// "target_velocity" function.
//
// Hint: p = p₀ + v₀t + ½at² (the third equation of kinematics)
//
// p.s. You can change your username by clicking on it at the top of the page.

use oort_api::prelude::*;

fn degree_to_radian(deg: f64) -> f64 {
    deg * (PI / 180.0)
}

const BULLET_SPEED: f64 = 1000.0; // m/s

pub struct Ship {
    prev_v: Vec2, // target v from previous tick
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            prev_v: vec2(0.0, 0.0),
        }
    }

    fn calculate_p1(&mut self) -> Vec2 {
        let p0 = target();
        let v = target_velocity();
        let d = target() - position(); // distance to target
        let t = d.length() / BULLET_SPEED; // how long before our bullets reach the target
        
        // calculate the acceleration. 
        // Note: TICK_LENGTH is the amount of seconds of a single tick
        // Since we are checking 'v' every tick, then this is the amount of time since the last time we updated 'v'
        let a = (v - self.prev_v) / TICK_LENGTH;

        // note that we now account for 'a'
        let mut p1 = p0 + v * t + 0.5 * a * t.powi(2);
        
        for _ in 0..100 {
            let d = p1 - position();
            let t = d.length() / BULLET_SPEED;
            p1 = p0 + v * t  + 0.5 * a * t.powi(2);
        }

        // At the end of our calculate_p1 method, we will update prev_v
        self.prev_v = v;

        p1
    }

    pub fn tick(&mut self) {
        let p1 = self.calculate_p1();
        let p1_angle = angle_diff(heading(), p1.angle());

        // draws a green line from our ship to the target ship
        // this is useful to visualize what is happening
        draw_line(position(), target(), 0x00ff00);

        // draws a cyan line to p1 of the target
        // this is where we should be aiming
        draw_line(position(), p1, 0x47cbe6);

        // Only fire if we are facing p1
        // Note: everything is in floats, so p1_angle will never be exactly 0
        if p1_angle.abs() < degree_to_radian(0.05) {
            fire(0); // this tell the ship to fire weapon number '0'
        }

        // Turn to face the target
        turn(p1_angle * 100.0);
    }
}
