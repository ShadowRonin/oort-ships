// Tutorial: Search
// Destroy the enemy ship. It is initially outside of your radar range.
// Hint: The set_radar_width() function can be used to create a tighter radar
// beam that's effective at longer distances.

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
    max_range: f64,
    max_velocity: f64,

    number_of_ticks_skipped: u64,

    debug_turn: bool,
    debug_fire: bool,
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            scan_result: Option::None,
            prev_scan_result: Option::None,

            seconds_before_using_turn: TICK_LENGTH,
            firing_offset: degree_to_radian(0.5),
            max_range: 3_000.0,
            max_velocity: 5.0 * max_forward_acceleration(),

            number_of_ticks_skipped: 0,

            debug_turn: false,
            debug_fire: true,
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
        // Make the radar skinny, so that it has a longer range
        set_radar_width(degree_to_radian(5.0));

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

    fn fire(&self, target: Vec2) {
        let distance_to_target = target.distance(position());

        
        // What our heading needs to be to face the target
        let target_heading = (target - position()).angle();
        
        // how far we need to turn
        let angle_to_turn = angle_diff(heading(), target_heading);

        if self.debug_fire {
            debug!("distance to target: {}", distance_to_target);
            debug!("angle to turn: {}", radian_to_degree(angle_to_turn));
        }

        // fire if in range and firing arc
        if distance_to_target < self.max_range && angle_to_turn.abs() < self.firing_offset {
            fire(0); // this tell the ship to fire weapon number '0'
        }
    }
    
    fn intercept_target(&self, target: Vec2) {
        let to_target = target - position();
        
        let time_to_target = to_target.length() / velocity().length();
        let time_to_stop = velocity().length() / max_forward_acceleration();
        
        if time_to_target < time_to_stop || to_target.length() < 1_000.0 {
            // break
            accelerate(-1.0 * velocity().normalize() * max_forward_acceleration());
        } else {
            // accelerate
            let direction_to_target = (target - position()).normalize();
            accelerate(direction_to_target * max_forward_acceleration());
        }
    }

    fn should_skip_tick(&mut self) -> bool {
        let distance_to_target = self.scan_result.as_ref().unwrap().position.distance(position());

        // skip if they are out of range and we are max velocity
        if distance_to_target > self.max_range && velocity().length() >= self.max_velocity && self.number_of_ticks_skipped < 10 {
            self.number_of_ticks_skipped += 1;
            return true;
        } else {
            self.number_of_ticks_skipped = 0;
            return false;
        }
    }
    
    pub fn tick(&mut self) {
        self.scan();

        if self.scan_result.is_some() && !self.should_skip_tick() {
            let p1 = self.calculate_p1();
            
            // What our heading needs to be to face p1
            let target_heading = (p1 - position()).angle();

            // draws a green line from our ship to the target ship
            // this is useful to visualize what is happening
            draw_line(position(), target(), 0x00ff00);

            // draws a cyan line to p1 of the target
            // this is where we should be aiming
            draw_line(position(), p1, 0x47cbe6);

            // Turn to face the target
            self.turn(target_heading);

            // Move towards the target
            self.intercept_target(p1);

            // Fire!
            self.fire(p1);
        }
    }
}

