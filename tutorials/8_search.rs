// Tutorial: Search
// Destroy the enemy ship. It is initially outside of your radar range.
// Hint: The set_radar_width() function can be used to create a tighter radar
// beam that's effective at longer distances.
use oort_api::prelude::*;

const BULLET_SPEED: f64 = 1000.0; // m/s

fn degree_to_radian(deg: f64) -> f64 {
    deg * (PI / 180.0)
}
fn radian_to_degree(r: f64) -> f64 {
    r * 180.0 / PI
}

fn linear(x1: f64, y1: f64, x2: f64, y2: f64) -> Box<dyn Fn (f64) -> f64> {
    let slope = (y2 - y1) / (x2 - x1);
    let intercept = y1 - slope * x1;

    Box::new(move |x: f64| slope * x + intercept)
}

struct Target {
    last_seen: f64,
    hits: Vec<ScanHit>,
    expire_after: f64, // Number of seconds before this target is no longer valid
}

impl Target {
    fn new(sr: ScanResult) -> Target {
        Target {
            last_seen: current_time(),
            hits: vec!(ScanHit::new(sr)),
            expire_after: (TICK_LENGTH * (degree_to_radian(360.0) / degree_to_radian(10.0))) + TICK_LENGTH * 4.0,
        }
    }

    fn match_last_seen(&self, scan: &ScanResult, debug: bool) -> bool {
        let last_scan = &self.hits.last().unwrap();
        let dt = current_time() - last_scan.time;
        let max_aceleration = 10.0 * max_forward_acceleration();
        
        let a = (vec2(scan.velocity.x, scan.velocity.y) - last_scan.result.velocity) / dt;
        if a.length() > max_aceleration { 
            if debug { debug!("acceleration"); }
            return false 
        }

        let predicted_position = self.future_position(current_time(), true);
        let a = (scan.position - self.position()) / dt;
        
        let v = scan.position - self.position();
        let max_v = self.velocity() + vec2(0.0, max_aceleration);

        if debug { 
            draw_triangle(predicted_position, 100.0, 0xeaed42); // yellow
            draw_square(predicted_position, max_aceleration * dt, 0xeaed42); // yellow
            draw_triangle(scan.position, 100.0, 0xc2330c); // red
    
            debug!("max a: {}", max_aceleration);
            debug!("a: {}", a.length());
            // let dv = scan.velocity - &self.hits.last().unwrap().result.velocity;
            // debug!("dv: {}", dv.length());
            // debug!("dp: {}", dp.length());
            // debug!("m_acc: {}", max_aceleration * dt);

            debug!("dt {}", dt);
            debug!("max v: {}", max_v.length());
            debug!("v: {}", v.length());
        }
        
        if v.length() > max_v.length() {
            if debug { debug!("vel"); }
            return false 
        }

        if debug { debug!("match"); }
        true
    }

    fn future_position(&self, t: f64, ignore_aceleration: bool) -> Vec2 {
        let scan1 = &self.hits.last().unwrap();
        let scan0 = &self.hits.get(self.hits.len() - 2);
        
        // Hint: p = p₀ + v₀t + ½at² (the third equation of kinematics)
        let p0 = scan1.result.position;
        let v = scan1.result.velocity;
        let a = 
            if ignore_aceleration || scan0.is_none() {vec2(0.0,0.0)} 
            else { scan1.aceleration(scan0.unwrap()) };
        let dt = t - scan1.time;

        p0 + (v * dt) + (0.5 * a * (dt * dt))
    }

    fn add_scan(&mut self, sr: ScanResult) {
        self.last_seen = current_time();
        self.hits.push(ScanHit::new(sr));
    }

    fn position(&self) -> Vec2 {
        let scan1 = &self.hits.last().unwrap();
        vec2(scan1.result.position.x, scan1.result.position.y)
    }

    fn velocity(&self) -> Vec2 {
        let scan1 = &self.hits.last().unwrap();
        vec2(scan1.result.velocity.x, scan1.result.velocity.y)
    }

    fn aceleration(&self) -> Vec2 {
        let scan1 = &self.hits.last().unwrap();
        let scan0 = &self.hits.get(self.hits.len() - 2);

        if let Some(scan0) = scan0 {
            scan1.aceleration(scan0)
        } else {
            vec2(0.0, 0.0)
        }
    }

    fn has_expired(&self) -> bool {
        current_time() - self.last_seen >= self.expire_after
    }

}

//struct ScanResult { position: Vec2, velocity: Vec2 }
struct ScanHit {
    result: ScanResult,
    time: f64,
}
impl ScanHit {
    fn new(result: ScanResult) -> ScanHit {
        ScanHit {
            time: current_time(),
            result
        }
    }

    fn aceleration(&self, hit: &ScanHit) -> Vec2 {
        let dt = self.time - hit.time;
        (self.result.velocity - hit.result.velocity) / dt
    }
}

pub struct Ship {
    closest_target: Option<Target>,
    search: bool,
    search_start: f64,
    number_targets: i64,

    debug_scan: bool,
    debug_fire: bool,
    debug_move: bool,
    debug_turn: bool,
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            closest_target: None,
            search: false,
            search_start: 0.0,
            number_targets: 0,

            debug_scan: false,
            debug_fire: true,
            debug_move: false,
            debug_turn: false,
        }
    }

    fn scan(&mut self) {
        if self.debug_scan { 
            debug!("Number of targets: {}", self.number_targets);
        }

        // Process scan
        if let Some(s) = scan() {
            if self.closest_target.is_some() {
                let t = self.closest_target.as_mut().unwrap();
                if t.match_last_seen(&s, self.debug_scan) {
                    t.add_scan(s);
                } else if s.position.distance(position()) < t.position().distance(position()) {
                    self.number_targets += 1;
                    self.closest_target = Some(Target::new(s));
                }
            } else {
                self.number_targets += 1;
                self.closest_target = Some(Target::new(s));
            }
        }

        // Remove expired target
        if self.closest_target.as_ref().is_some() && self.closest_target.as_ref().unwrap().has_expired() {
            self.closest_target = None;
        }

        // Move rader
        if self.search {
            if radar_heading() < degree_to_radian(10.0) && self.closest_target.is_some() {
                self.search = false;
            } else {
                self.search_start = self.search_start + degree_to_radian(10.0);
                set_radar_heading(self.search_start);
            }
        } else if self.closest_target.is_some() {
            let fp = self.closest_target.as_ref()
                .unwrap()
                .future_position(current_time() + TICK_LENGTH, false);

            set_radar_heading((fp - position()).angle());

            if position().distance(fp) < 100.0 {
                set_radar_width(degree_to_radian(120.0));
            } else {
                set_radar_width(degree_to_radian(30.0));
            }
        } else {
            self.search = true;
            self.search_start = 10.0;
            set_radar_heading(self.search_start);
            set_radar_width(degree_to_radian(10.0));
        }
    }

    fn calc_future_target(&self, depth_of_calc: u32) -> Vec2 {
        // Hint: p = p₀ + v₀t + ½at² (the third equation of kinematics)
        let target = self.closest_target.as_ref().unwrap();
        
        let p0 = target.position();
        let v = target.velocity();
        let mut t = (target.position() - position()).length() / BULLET_SPEED;
        let a = target.aceleration();
        let mut pf = p0 + (v * t) + (0.5 * a * (t * t));

        // TODO: maybe just have some percent offset for t, instead of recalculating
        for _ in [..depth_of_calc] {
            t = (pf - position()).length() / BULLET_SPEED;
            pf = p0 + v * t + 0.5 * a * t * t;
        }

        pf
    }

    fn turn(&self, target: Vec2) {
        // rand offset, to help account for them changing acceleration
        const max_jitter: f64 = 2.0;
        let jitter = degree_to_radian(rand(-1.0 * max_jitter, max_jitter));

        let target_angle = (target - position()).angle();
        let target_angle_with_jitter = target_angle + jitter;
        let angle_diff = angle_diff(
            heading(), 
            target_angle
        );

        let v = angular_velocity();
        let seconds_to_stop = v / max_angular_acceleration();
        let seconds_to_target = angle_diff / v;

        let should_break = 
            seconds_to_stop > 0.0 
            && seconds_to_target > 0.0
            && seconds_to_stop >= seconds_to_target;

        if self.debug_turn {
                
            debug!("target: {}; heading: {};", radian_to_degree(target_angle), radian_to_degree(heading()));
            debug!("jitter: {}; diff: {};", radian_to_degree(jitter), radian_to_degree(angle_diff));
            debug!("v: {}; max a: {};", radian_to_degree(v), radian_to_degree(max_angular_acceleration()));
            debug!("to stop: {}; to target: {}; break: {}", seconds_to_stop, seconds_to_target, should_break);
        }

        if should_break {
            // break!
            if v > 0.0 {
                torque(-1.0 * max_angular_acceleration());
            } else {
                torque(max_angular_acceleration());
            }
        } else {
            if angle_diff > 0.0 {
                torque(max_angular_acceleration());
            } else {
                torque(-1.0 * max_angular_acceleration());
            }
        }
    }
    
    fn fire(&self, target: Vec2) {
        const max_range: f64 = 2_000.0;
        let angle = (target - position()).angle();
        let angle_diff = angle_diff(
            heading(), 
            angle
        );
        let distance = target.distance(position());
        
        
        const d1: f64 = 1_000.0;
        let a1: f64 = degree_to_radian(4.0);
        const d2: f64 = 5_000.0;
        let a2: f64 = degree_to_radian(4.0);

        // draw_line(position(), position() + vec2(0.0, d1).rotate(a1), 0xed85dc);
        // draw_line(position(), position() + vec2(0.0, d1).rotate(-1.0 * a1), 0xed85dc);

        // draw_line(position(), position() + vec2(d2, 0.0).rotate(a2), 0xed85dc);
        // draw_line(position(), position() + vec2(d2, 0.0).rotate(-1.0 * a2), 0xed85dc);
        
        
        // TODO: figure out why it is never in the fireing arc
            // a: not actually facing the right way...
            // b: firing arc calculation is off
        let max_angle = linear(d1,a1,d2,a2)(distance);
        let max_angle = degree_to_radian(3.0);
        let deg_dif = radian_to_degree(angle_diff);

        draw_line(position(), position() + vec2(1000.0, 0.0).rotate(heading()), 0xebedf0);
        
        let in_range = distance < max_range;
        let in_firing_arc = angle_diff.abs() < max_angle;
        
        if self.debug_fire {
            debug!("distance: {}; max range: {}; {}", distance, max_range, in_range);
            debug!("angle dif: {}; max angle: {}; {}", deg_dif, radian_to_degree(max_angle), in_firing_arc);
            draw_line(position(), position() + vec2(max_range, 0.0).rotate(angle + max_angle), 0xed85dc);
            draw_line(position(), position() + vec2(max_range, 0.0).rotate(angle - max_angle), 0xed85dc);
        }
        
        // TODO: handle differnt guns for different ship types (some have turrets!)
        aim(0, angle);
        if in_range && in_firing_arc {
            fire(0);
        }
    }

    fn move_ship(&self, target: Vec2) {
        self.turn(target);

        let dp = target - position();
        
        let heading_normilized = vec2(1.0, 0.0).rotate(heading());
        let v = velocity();
        let v_normalized = velocity().normalize(); // this is NaN if v = 0
        let target_v_normalized = dp.normalize();
        let v_normalized_diff = target_v_normalized - v_normalized;
        
        let dv =
        if v.length() > 0.0 { (heading_normilized + v_normalized_diff) * max_forward_acceleration() }
        else { heading_normilized * max_forward_acceleration() };

        const passing_speed: f64 = 100.0;
        let seconds_to_accelerate_to_passing_speed = passing_speed / max_forward_acceleration();
        let seconds_to_stop = v.length() / max_forward_acceleration();
        let seconds_to_passing_speed = seconds_to_stop - seconds_to_accelerate_to_passing_speed;
        let seconds_to_intercept = dp.length() / v.length();
        let should_break = seconds_to_passing_speed >= seconds_to_intercept;
        
        
        if self.debug_move {
            debug!("should break: {}", should_break);
            debug!("v: {}", velocity());
            debug!("dv: {}", dv);

        }

        if should_break {
            accelerate(-1.0 * v_normalized * max_forward_acceleration());
        } else {
            accelerate(dv);
        }
    }
    
    pub fn tick(&mut self) {
        self.scan();

        if let Some(t) = self.closest_target.as_ref() {
            draw_line(position(), t.position(), 0x00ff00);
    
            let target_in_time =  self.calc_future_target(100);

            draw_line(position(), target_in_time, 0x9c2488);
    
            self.fire(target_in_time);
            self.move_ship(target_in_time);
        } else {
            let a = velocity() * max_forward_acceleration() * -1.0;
            accelerate(a);
        }
    }
}
