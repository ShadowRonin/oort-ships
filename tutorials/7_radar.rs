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

const BULLET_SPEED: f64 = 1000.0; // m/s

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
            expire_after: 0.5,
        }
    }

    fn match_last_seen(&self, scan: &ScanResult) -> bool {
        let max_aceleration = 2.0 * max_forward_acceleration();

        let last_scan = &self.hits.last().unwrap();
        let dt = current_time() - last_scan.time;
        
        let a = (vec2(scan.velocity.x, scan.velocity.y) - last_scan.result.velocity) / dt;
        if a.length() > max_aceleration { return false }

        let predicted_position = self.future_position(current_time(), true);
        let dp = predicted_position - scan.position;
        if dp.length() > (max_aceleration * dt) { return false }

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

fn degree_to_radian(deg: f64) -> f64 {
    deg * (PI / 180.0)
}

pub struct Ship {
    closest_target: Option<Target>,
    search: bool,
    search_start: f64,
}

impl Ship {
    pub fn new() -> Ship {
        Ship {
            closest_target: None,
            search: false,
            search_start: 0.0,
        }
    }

    fn scan(&mut self) {
        // Process scan
        if let Some(s) = scan() {
            if self.closest_target.is_some() {
                let t = self.closest_target.as_mut().unwrap();
                if t.match_last_seen(&s) {
                    t.add_scan(s);
                } else if s.position.distance(position()) < t.position().distance(position()) {
                    self.closest_target = Some(Target::new(s));
                }
            } else {
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
        } else {
            self.search = true;
            self.search_start = 10.0;
            set_radar_heading(self.search_start);
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

    // TODO: properly calcurlate how much torque to add to "perfectly" turn the ship
    fn turn(&self, angle: f64) {
        if angle.abs() > degree_to_radian(60.0) {
            if angle < 0.0 {
                torque(-1.0 * max_angular_acceleration())
            } else {
                torque(max_angular_acceleration())
            }
        } else {
            turn(angle);
        }
    }
    
    pub fn tick(&mut self) {
        self.scan();

        if let Some(t) = self.closest_target.as_ref() {
            draw_line(position(), t.position(), 0x00ff00);
    
            let target_in_time = self.calc_future_target(100);
            let target_in_time_ang = angle_diff(
                heading(), 
                // target_in_time.angle()
                (target_in_time - position()).angle()
            );

            draw_line(position(), target_in_time, 0x47cbe6);
    
            if target_in_time_ang.abs() < degree_to_radian(5.0) {
                fire(0);
            }

            // TODO: refine my improved turn to actually be better
            //self.turn(target_in_time_ang);
            turn(target_in_time_ang * 1000.0);

            let a = vec2(0.1 * max_forward_acceleration(), 0.0).rotate((target_in_time - position()).angle());
            accelerate(a);
        } else {
            let a = velocity() * max_forward_acceleration() * -1.0;
            accelerate(a);
        }
    }
}
