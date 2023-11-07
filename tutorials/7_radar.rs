// Tutorial: Radar
// Destroy the enemy ships. Use your radar to find them.
// Hint: Press 'g' in-game to show where your radar is looking.
// Hint: Press 'n' to single-step.
// Hint: Use the set_radar_heading() function to keep your radar pointed at a
// target, or to search for a new one.
//
// Join the Discord at https://discord.gg/vYyu9EhkKH for Oort discussion and
// tournament results.

use oort_api::prelude::*;

fn degree_to_radian(deg: f64) -> f64 {
    deg * (PI / 180.0)
}
fn radian_to_degree(r: f64) -> f64 {
    r * 180.0 / PI
}

const BULLET_SPEED: f64 = 1000.0; // m/s

pub struct Ship {
    scan_result: Option<ScanResult>,
    prev_scan_result: Option<ScanResult>,

    seconds_before_using_turn: f64,
    firing_offset: f64,

    debug_turn: bool,
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            scan_result: Option::None,
            prev_scan_result: Option::None,

            seconds_before_using_turn: TICK_LENGTH,
            firing_offset: degree_to_radian(0.05),


            debug_turn: false,
        }
    }

    fn calculate_p1(&mut self) -> Vec2 {
        let target = self.scan_result.as_ref().unwrap();
        let p0 = target.position;
        let v = target.velocity;
        let d = p0 - position(); // distance to target
        let t = d.length() / BULLET_SPEED; // how long before our bullets reach the target
        
        // calculate the acceleration. 
        // Note: TICK_LENGTH is the amount of seconds of a single tick
        // Since we are checking 'v' every tick, then this is the amount of time since the last time we updated 'v'
        let a: Vec2;
        if self.prev_scan_result.is_some() {
            let prev_v = self.prev_scan_result.as_ref().unwrap().velocity;
            a = (v - prev_v) / TICK_LENGTH;
        } else {
            a = vec2(0.0, 0.0);
        }

        // note that we now account for 'a'
        let mut p1 = p0 + v * t + 0.5 * a * t.powi(2);
        
        for _ in 0..100 {
            let d = p1 - position();
            let t = d.length() / BULLET_SPEED;
            p1 = p0 + v * t  + 0.5 * a * t.powi(2);
        }

        p1
    }

    fn scan(&mut self) {
        // Attempt to get info from our radar
        if let Some(scan) = scan() {
            // Turns the radar towards the target
            let towards_scan  = scan.position - position();
            set_radar_heading(towards_scan.angle());

            // Update the scan results
            self.prev_scan_result = std::mem::replace(&mut self.scan_result, Some(scan));
        } else {
            // Turns the radar in a circle until we find a target
            set_radar_heading(radar_heading() + radar_width());

            // Remove old scans, as we have lost, or destroyed, the target
            self.prev_scan_result = Option::None;
            self.scan_result = Option::None;
        }
    }

    // Use torque to turn faster
    fn turn(&self, target_heading: f64) {
        // While `turn` turns you to a given heading
        // `torque` accelerates you in a spin
        // We can calculate the optimal amount to `torque`
        // Allowing us to turn much faster

        // How far we need to turn
        let angle_diff = angle_diff(heading(), target_heading);

        // How fast we are spinning
        // > 0, we are spinning counter-clockwise
        // < 0, we are spinning clockwise
        let v = angular_velocity();

        // How long it will takes us to kill our angular velocity
        let seconds_to_stop = v.abs() / max_angular_acceleration();

        // How long it will take us to reach the target heading giving the current v
        let seconds_to_target_heading = angle_diff / v;

        // We should start breaking if we would overshoot our target otherwise
        let should_break = 
            seconds_to_stop > 0.0 
            && seconds_to_target_heading > 0.0
            && seconds_to_stop >= seconds_to_target_heading;

        // Info to help us debug
        if self.debug_turn {
            debug!("target: {}; heading: {};", radian_to_degree(target_heading), radian_to_degree(heading()));
            debug!("degrees to turn: {};", radian_to_degree(angle_diff));
            debug!("v: {}; max a: {};", radian_to_degree(v), radian_to_degree(max_angular_acceleration()));
            debug!("to stop: {}; to target: {}; break: {}", seconds_to_stop, seconds_to_target_heading, should_break);
        }

        if should_break {
            // Break!
            if v > 0.0 {
                torque(-1.0 * max_angular_acceleration());
            } else {
                torque(max_angular_acceleration());
            }
        } else {
            let a = max_angular_acceleration();
            
            // Using torque can be hard once we are close to our target
            // As when the target changes we tend to overshoot back and forth
            // Instead we will use `turn` for the last leg
            if seconds_to_target_heading < self.seconds_before_using_turn {
                turn(target_heading - heading());
            }

            // Spin!
            if angle_diff > 0.0 {
                torque(a);
            } else {
                torque(-1.0 * a);
            }
        }
    }

    pub fn tick(&mut self) {
        self.scan();

        if self.scan_result.is_some() {
            let p1 = self.calculate_p1();
            
            // What our heading needs to be to face p1
            let target_heading = (p1 - position()).angle();
            
            // how far we need to turn
            let angle_to_turn = angle_diff(heading(), target_heading);

            // draws a green line from our ship to the target ship
            // this is useful to visualize what is happening
            draw_line(position(), target(), 0x00ff00);

            // draws a cyan line to p1 of the target
            // this is where we should be aiming
            draw_line(position(), p1, 0x47cbe6);

            // Only fire if we are facing p1
            // Note: everything is in floats, so p1_angle will never be exactly 0
            if angle_to_turn.abs() < self.firing_offset {
                fire(0); // this tell the ship to fire weapon number '0'
            }

            // Turn to face the target
            self.turn(target_heading);
        }
    }
}

