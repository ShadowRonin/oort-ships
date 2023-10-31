// Tutorial: Radio
// Destroy the enemy ship. Your radar is broken, but a radio signal on channel
// 2 will give you its position and velocity.

use oort_api::prelude::*;

const BULLET_SPEED: f64 = 1000.0; // m/s

fn degree_to_radian(deg: f64) -> f64 {
    deg * (PI / 180.0)
}
fn radian_to_degree(r: f64) -> f64 {
    r * 180.0 / PI
}

fn estimate_future_position(p0: Vec2, v: Vec2, a: Vec2, dt: f64) -> Vec2 {
    p0 + (v * dt) + (0.5 * a * (dt * dt))
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

pub struct TargetEstimate {
    estimate_time: f64,
    created_time: f64, 
    position: Vec2,
    angle_error: f64,
}

pub struct Ship {
    max_range: f64,
    closest_target: Option<Target>,
    distance_to_target: f64,
    is_weapon_ready: bool,

    search: bool,
    search_start: f64,
    number_targets: i64,

    fire_offset_percent: f64,
    fire_offset_percent_increment: f64,

    ticks_since_last_check: u64,
    max_v: f64,

    debug_scan: bool,
    debug_fire: bool,
    debug_move: bool,
    debug_turn: bool,
    debug_future_target: bool,
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            max_range: 3_000.0,
            closest_target: None,
            distance_to_target: 0.0,
            is_weapon_ready: false,

            search: false,
            search_start: 0.0,
            number_targets: 0,

            fire_offset_percent: 0.0,
            fire_offset_percent_increment: 0.2,

            ticks_since_last_check: 0,
            max_v: 6.0 * max_forward_acceleration(),

            debug_scan: false,
            debug_fire: false,
            debug_move: false,
            debug_turn: false,
            debug_future_target: false,
        }
    }

    fn skip_tick(&mut self) -> bool {
        // oort only allows each ship 1,000,000 instructions
        // this allows as to skip intensive calculations at times
        // thereby saving instructions to be used when we are closer to our target
        let should_skip = self.distance_to_target > self.max_range * 1.3
        && self.ticks_since_last_check < 5
        && velocity().length() >= self.max_v;

        if should_skip {
            self.ticks_since_last_check = self.ticks_since_last_check + 1;
        } else {
            self.ticks_since_last_check = 0;
        }
        should_skip
    }

    fn radio(&mut self) {
        set_radio_channel(2);
        if let Some(msg) = receive() {
            let p = vec2(msg[0], msg[1]);
            self.distance_to_target = (p - position()).length();
            let s = ScanResult {
                position: p,
                velocity: vec2(msg[2], msg[3]),
                rssi: 0.0,
                snr: 0.0,
                class: Class::Fighter,
            };

            if self.closest_target.is_none() {
                self.closest_target = Some(Target::new(s));
            } else {
                let t = self.closest_target.as_mut().unwrap();
                t.add_scan(s);
            }
        } else {
            debug!("No message!");
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

    fn calc_future_target(&self, depth_of_calc: u32) -> TargetEstimate {
        // Hint: p = p₀ + v₀t + ½at² (the third equation of kinematics)
        let target = self.closest_target.as_ref().unwrap();
        
        let mut t = (target.position() - position()).length() / BULLET_SPEED;
        let mut pf = estimate_future_position(target.position(), target.velocity(), vec2(0.0,0.0), t);

        // TODO: maybe just have some percent offset for t, instead of recalculating
        for _ in [..depth_of_calc] {
            t = (pf - position()).length() / BULLET_SPEED;
            pf = estimate_future_position(target.position(), target.velocity(), target.aceleration(), t);
        }

        let ninety_deg_angle = (position() - target.position()).rotate(degree_to_radian(90.0)).normalize();
        let offset_a = ninety_deg_angle * max_forward_acceleration();
        let pf_offset = estimate_future_position(target.position(), target.velocity(), offset_a, t);
        let pf_offset2 = estimate_future_position(target.position(), target.velocity(), -1.0 * offset_a, t);
        
        // let pf = vec2((pf_offset.x + pf_offset2.x) / 2.0, (pf_offset.y + pf_offset2.y) / 2.0);

        let m = position();
        let blue = m - pf;
        let red = m - pf_offset;
        let gray = pf - pf_offset;
        
        // yellow green angle
        let Y = pf_offset - target.position();
        let G = position() - target.position();
        let yg = (Y.dot(G) / (Y.length() * G.length())).acos();

        let angles = calculate_angles(m, pf, pf_offset);

        let deg_blue_gray = radian_to_degree(angles.1);
        let deb_blue_red = radian_to_degree(angles.0);
        let deb_red_gray = radian_to_degree(angles.2);
        let deg_total = deg_blue_gray + deb_blue_red + deb_red_gray;
        
        let angles2 = calculate_angles(m, pf, pf_offset2);
        
        let angle_error = angles.0.max(angles2.0).max(degree_to_radian(1.0));


        if self.debug_future_target {
            debug!("(blue) m - pf = {}", blue.length());
            debug!("(red)  m - pfo = {}", red.length());
            debug!("(gray) pf - pfo = {}", gray.length());
            debug!("deg_blue_red = {deb_blue_red}");
            debug!("deg_blue_gray = {deg_blue_gray}");
            debug!("deg_red_gray = {deb_red_gray}");
            debug!("deg_total = {deg_total}");
            debug!("angle_error = {}", radian_to_degree(angle_error));

            
            draw_line(m, pf, 0x0037fc); // blue
            draw_line(m, pf_offset, 0xbd0416); // red
            draw_line(pf, pf_offset, 0x9e9e9e); // gray

            draw_line(pf, pf_offset2, 0x9e9e9e); // gray
            draw_line(m, pf_offset2, 0xbd0416); // red


            debug!("deg yellow green = {}", radian_to_degree(yg));
            draw_line(target.position(), pf, 0xf279ae); // pink
            draw_line(target.position(), pf_offset, 0xe3e31b); // yellow
            draw_line(target.position(), target.position() + (offset_a * 1000.0), 0xdb9523); // orange
        }
        
        TargetEstimate {
            estimate_time: current_time() + t,
            created_time: current_time(), 
            position: pf,
            angle_error,
        }
    }

    fn get_fire_offset(&mut self, angle_error: f64) -> f64 {
        let offset = angle_error * self.fire_offset_percent;
        
        // Update offset if we can fire this tick
        if self.is_weapon_ready {
            self.fire_offset_percent = self.fire_offset_percent + self.fire_offset_percent_increment;
            if self.fire_offset_percent >= 1.0 || self.fire_offset_percent <= -1.0 {
                self.fire_offset_percent_increment = -1.0 * self.fire_offset_percent_increment;
            }
        }
        
        offset
    }
    
    fn turn(&mut self, target: &TargetEstimate) {
        // rand offset, to help account for them changing acceleration
        let jitter = self.get_fire_offset(target.angle_error);

        let target_angle = (target.position - position()).angle();
        let target_angle_with_jitter = target_angle + jitter;
        let angle_diff = angle_diff(
            heading(), 
            target_angle_with_jitter
        );

        let v = angular_velocity();
        let seconds_to_stop = v.abs() / max_angular_acceleration();
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
            let a = max_angular_acceleration();
            if seconds_to_target < 1.0 {
                turn(target_angle - heading());
            }

            if angle_diff > 0.0 {
                torque(a);
            } else {
                torque(-1.0 * a);
            }
        }
    }
    
    fn fire(&self, _target: &TargetEstimate) {
        if self.is_weapon_ready {
            fire(0);
        }
    }

    fn move_ship(&self, target: &TargetEstimate) {
        let dp = target.position - position();
        
        let heading_normilized = vec2(1.0, 0.0).rotate(heading());
        let v = velocity();
        let v_normalized = velocity().normalize(); // this is NaN if v = 0
        let target_v_normalized = dp.normalize();
        let v_normalized_diff = target_v_normalized - v_normalized;
        
        let dv =
        if v.length() > 0.0 { (heading_normilized + v_normalized_diff) * max_forward_acceleration() }
        else { heading_normilized * max_forward_acceleration() };

        const passing_speed: f64 = 200.0;
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
    
    fn update_weapon_readiness(&mut self, target: &TargetEstimate) {
        let d = (position() - target.position).length();
        let angle = (target.position - position()).angle();
        let angle_diff = angle_diff(
            heading(), 
            angle
        );

        let is_reloading = reload_ticks(0) > 0;
        let in_range = d < self.max_range;
        let in_firing_arc = angle_diff.abs() <= target.angle_error;

        if self.debug_fire {
            debug!("is reloading: {is_reloading}");
            debug!("in range: {in_range}; distance: {}; max range: {};", d, self.max_range);
            debug!("in arc: {in_firing_arc}; angle dif: {}; angle error: {};", radian_to_degree(angle_diff), radian_to_degree(target.angle_error));
            draw_line(position(), position() + vec2(self.max_range, 0.0).rotate(angle + target.angle_error), 0xed85dc);
            draw_line(position(), position() + vec2(self.max_range, 0.0).rotate(angle - target.angle_error), 0xed85dc);
        }

        self.is_weapon_ready = !is_reloading && in_range && in_firing_arc;
    }
    
    pub fn tick(&mut self) {
        // self.scan();
        self.radio();

        // Need to stretch out the max(1m) number of instructions
        if !self.skip_tick() {
            if let Some(t) = self.closest_target.as_ref() {
                draw_line(position(), t.position(), 0x00ff00);
        
                let target_in_time =  self.calc_future_target(100);
                self.update_weapon_readiness(&target_in_time);
    
                draw_line(position(), target_in_time.position, 0x9c2488);
        
                self.turn(&target_in_time);
                self.fire(&target_in_time);
                self.move_ship(&target_in_time);
            } else {
                let a = velocity() * max_forward_acceleration() * -1.0;
                accelerate(a);
            }
        } else {
            debug!("SKIP TICK");
        }
    }
}

fn calculate_angles(a: Vec2, b: Vec2, c: Vec2) -> (f64, f64, f64) {
    fn calc_first_angle(a: Vec2, b: Vec2, c: Vec2) -> f64 {
        let B = b - a;
        let C = c - a;
        (B.dot(C) / (B.length() * C.length())).acos()
    }
    
    (
        calc_first_angle(a, b, c),
        calc_first_angle(b, a, c),
        calc_first_angle(c, a, b),
    )
}
