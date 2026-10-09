#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LapTimes {
    pub current_lap: i32,
    pub last_lap_time: f32,
    pub best_lap_time: f32,
    pub lap_start_time: f32,
}

impl LapTimes {
    pub fn start_lap(&mut self, lap: i32, lap_start_time: f32) {
        let lap_time = lap_start_time - self.last_lap_time;

        self.best_lap_time = if lap <= 1 {
            -1.0
        } else if self.best_lap_time < 0.0 || lap_time < self.best_lap_time {
            lap_time
        } else {
            self.best_lap_time
        };

        self.current_lap = lap;
        self.lap_start_time = lap_start_time;
        self.last_lap_time = lap_start_time;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_lap_has_no_best_time() {
        let mut laps = LapTimes::default();
        laps.start_lap(1, 3.0);

        assert_eq!(laps.current_lap, 1);
        assert_eq!(laps.best_lap_time, -1.0);
        assert_eq!(laps.lap_start_time, 3.0);
    }

    #[test]
    fn the_best_time_is_the_fastest_finished_lap() {
        let mut laps = LapTimes::default();
        laps.start_lap(1, 3.0);
        laps.start_lap(2, 63.0);
        assert_eq!(laps.best_lap_time, 60.0);

        laps.start_lap(3, 118.0);
        assert_eq!(laps.best_lap_time, 55.0);

        laps.start_lap(4, 180.0);
        assert_eq!(laps.best_lap_time, 55.0);
        assert_eq!(laps.current_lap, 4);
    }
}
