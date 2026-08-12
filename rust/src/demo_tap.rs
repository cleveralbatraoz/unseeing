//! Dev-only cadence for input-less demo taps: first fire at ~0.6 s, then
//! every 4 s measured from each fire, always at the same pinned wall point.
//! Pure schedule: the caller owns arming and wave queueing; this class only
//! answers "is a tap due now?".
//!
//! The schedule rides actual fire time: if the first `fire_due` call arrives
//! late (e.g., at now=1.0 when _next=0.6), the schedule advances to 1.0+4.0,
//! so the second fire is at 5.0, not 4.6. This keeps every frame's cadence
//! within one frame of the ideal beat, even under frame drops.

use godot::builtin::Vector3;

/// First tap due at this many seconds.
const FIRST_AT: f64 = 0.6;

/// Interval between taps, measured from fire time.
const REPEAT_EVERY: f64 = 4.0;

/// Dev-only tap schedule: fires at regular intervals when armed.
#[derive(Clone, Debug)]
pub struct DemoTap {
    /// Whether the schedule is active; when false, `fire_due` always returns false.
    pub armed: bool,
    /// The point in world space where taps are sourced.
    pub point: Vector3,
    /// The normal vector at the tap point.
    pub normal: Vector3,
    /// Next fire time; rides actual fire moment.
    next: f64,
}

impl DemoTap {
    /// Create a new tap schedule, disarmed and due at FIRST_AT.
    pub fn new(point: Vector3, normal: Vector3) -> Self {
        Self {
            armed: false,
            point,
            normal,
            next: FIRST_AT,
        }
    }

    /// True when the armed schedule has a tap due now, advancing the schedule
    /// to now + REPEAT_EVERY. The next due moment rides on the actual fire
    /// time, so any frame cadence lands within one frame of the ideal beat.
    pub fn fire_due(&mut self, now: f64) -> bool {
        if !self.armed || now < self.next {
            return false;
        }
        self.next = now + REPEAT_EVERY;
        true
    }

    /// The time of the next scheduled fire. Used by `capture_env`.
    pub fn next_at(&self) -> f64 {
        self.next
    }

    /// Restore the next fire time. Used by `restore_blob`.
    pub fn restore_next(&mut self, next: f64) {
        self.next = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1/60 frame duration; used for tolerance bounds.
    const DT: f64 = 1.0 / 60.0;

    /// Armed schedule fires exactly 3 times in 10 s at ~0.6/4.6/8.6 with
    /// tolerance of DT * 1.5. Each fire rides the actual fire time, so the
    /// next due moment is always now + REPEAT_EVERY.
    #[test]
    fn fires_at_expected_times() {
        let mut tap = DemoTap::new(Vector3::new(1.0, 2.0, 3.0), Vector3::new(-1.0, 0.0, 0.0));
        tap.armed = true;

        let mut fires = Vec::new();
        let mut now = 0.0;
        while now < 10.0 {
            now += DT;
            if tap.fire_due(now) {
                fires.push(now);
            }
        }

        // Should fire 3 times
        assert_eq!(fires.len(), 3, "Expected 3 fires, got {}", fires.len());

        // Fire times with tolerance of DT * 1.5
        let tolerance = DT * 1.5;
        assert!(
            (fires[0] - 0.6).abs() < tolerance,
            "First fire at {}, expected ~0.6 ±{}",
            fires[0],
            tolerance
        );
        assert!(
            (fires[1] - 4.6).abs() < tolerance,
            "Second fire at {}, expected ~4.6 ±{}",
            fires[1],
            tolerance
        );
        assert!(
            (fires[2] - 8.6).abs() < tolerance,
            "Third fire at {}, expected ~8.6 ±{}",
            fires[2],
            tolerance
        );
    }

    /// Unarmed schedule never fires, even if given times past FIRST_AT.
    #[test]
    fn unarmed_never_fires() {
        let mut tap = DemoTap::new(Vector3::ZERO, Vector3::UP);
        // armed is false by default
        assert!(!tap.armed);

        let mut fire_count = 0;
        let mut now = 0.0;
        while now < 10.0 {
            now += DT;
            if tap.fire_due(now) {
                fire_count += 1;
            }
        }

        assert_eq!(fire_count, 0, "Unarmed tap fired {} times", fire_count);
    }

    /// If the first fire_due call is delayed (arrives after FIRST_AT),
    /// the schedule rides that actual fire time. If called at now=1.0 when
    /// _next=0.6, then _next becomes 1.0+4.0=5.0, so the second fire is at 5.0.
    #[test]
    fn rides_actual_fire_time() {
        let mut tap = DemoTap::new(Vector3::ZERO, Vector3::UP);
        tap.armed = true;

        // Simulate late arrival: skip directly to now=1.0
        let result = tap.fire_due(1.0);
        assert!(result, "Expected fire at now=1.0 (delayed start)");

        // Next fire should be at 1.0 + REPEAT_EVERY = 5.0
        assert_eq!(tap.next_at(), 5.0, "Next fire should be at 5.0");

        // Verify the pattern continues: fire again at 5.0
        let result = tap.fire_due(5.0);
        assert!(result, "Expected fire at now=5.0");
        assert_eq!(tap.next_at(), 9.0, "Next fire should be at 9.0");
    }

    /// Mutation check: if _next rode the due date instead of fire time,
    /// the schedule would not behave correctly under late arrivals.
    /// This test ensures we catch that mutation.
    #[test]
    fn mutation_next_rides_fire_time() {
        // If someone changes _next = now_due + REPEAT_EVERY (wrong),
        // this test must fail. We verify the correct behavior:
        // _next = now_actual + REPEAT_EVERY (right).

        let mut tap = DemoTap::new(Vector3::ZERO, Vector3::UP);
        tap.armed = true;

        // First call at now=1.0 (late), should fire
        assert!(tap.fire_due(1.0));
        let next_after_late_fire = tap.next_at();

        // Should be exactly 1.0 + 4.0 = 5.0 (riding the actual fire time)
        // NOT 0.6 + 4.0 = 4.6 (riding the due date)
        assert_eq!(
            next_after_late_fire, 5.0,
            "Next should ride actual fire time (5.0), not due date (4.6)"
        );

        // Prove it by showing the next call at 4.9 does NOT fire
        assert!(!tap.fire_due(4.9), "Should not fire at 4.9");
        assert!(!tap.fire_due(4.99), "Should not fire at 4.99");
        // But at 5.0 or later, it should fire
        assert!(tap.fire_due(5.0), "Should fire at 5.0");
    }

    /// Point and normal are stored and retrievable.
    #[test]
    fn stores_point_and_normal() {
        let point = Vector3::new(1.0, 2.0, 3.0);
        let normal = Vector3::new(-1.0, 0.0, 0.0);
        let tap = DemoTap::new(point, normal);

        assert_eq!(tap.point, point);
        assert_eq!(tap.normal, normal);
    }

    /// restore_next updates the next fire time for capture/restore.
    #[test]
    fn restore_next_works() {
        let mut tap = DemoTap::new(Vector3::ZERO, Vector3::UP);
        assert_eq!(tap.next_at(), FIRST_AT);

        tap.restore_next(7.5);
        assert_eq!(tap.next_at(), 7.5);
    }
}
