#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SprintSettings {
    pub walk_speed: f32,
    pub fixed_tick_time: f32,
    pub sprint_time: f32,
    pub sprint_speed: f32,
    pub recovering_walk_speed: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SprintUpdate {
    pub max_walk_speed: Option<f32>,
    pub sprint_state_changed: Option<bool>,
    pub meter_percent: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SprintState {
    pub sprinting: bool,
    pub recovering: bool,
    pub meter: f32,
}

impl SprintState {
    pub fn begin(settings: &SprintSettings) -> (Self, SprintUpdate) {
        let state = SprintState {
            meter: settings.sprint_time,
            ..Default::default()
        };

        let update = SprintUpdate {
            max_walk_speed: Some(settings.walk_speed),
            ..Default::default()
        };

        (state, update)
    }

    pub fn start_sprint(&mut self, settings: &SprintSettings) -> SprintUpdate {
        self.set_sprinting(true, settings.sprint_speed)
    }

    pub fn end_sprint(&mut self, settings: &SprintSettings) -> SprintUpdate {
        self.set_sprinting(false, settings.walk_speed)
    }

    fn set_sprinting(&mut self, sprinting: bool, speed: f32) -> SprintUpdate {
        self.sprinting = sprinting;

        if self.recovering {
            return SprintUpdate::default();
        }

        SprintUpdate {
            max_walk_speed: Some(speed),
            sprint_state_changed: Some(sprinting),
            meter_percent: None,
        }
    }

    pub fn fixed_tick(&mut self, speed: f64, settings: &SprintSettings) -> SprintUpdate {
        let mut update = SprintUpdate::default();

        if self.sprinting && !self.recovering && speed > f64::from(settings.walk_speed) {
            if self.meter > 0.0 {
                self.meter = (self.meter - settings.fixed_tick_time).max(0.0);

                if self.meter <= 0.0 {
                    self.recovering = true;

                    update.max_walk_speed = Some(settings.recovering_walk_speed);
                }
            }
        } else {
            self.meter = (self.meter + settings.fixed_tick_time).min(settings.sprint_time);

            if self.meter >= settings.sprint_time {
                self.recovering = false;

                update.max_walk_speed = Some(if self.sprinting {
                    settings.sprint_speed
                } else {
                    settings.walk_speed
                });

                update.sprint_state_changed = Some(self.sprinting);
            }
        }

        update.meter_percent = Some(self.meter / settings.sprint_time);

        update
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTINGS: SprintSettings = SprintSettings {
        walk_speed: 250.0,
        fixed_tick_time: 0.5,
        sprint_time: 1.0,
        sprint_speed: 600.0,
        recovering_walk_speed: 150.0,
    };

    #[test]
    fn begins_walking_with_a_full_meter() {
        let (state, update) = SprintState::begin(&SETTINGS);
        assert_eq!(state.meter, 1.0);
        assert!(!state.sprinting && !state.recovering);
        assert_eq!(update.max_walk_speed, Some(250.0));
    }

    #[test]
    fn sprinting_burns_the_meter_until_recovery() {
        let (mut state, _) = SprintState::begin(&SETTINGS);
        let update = state.start_sprint(&SETTINGS);
        assert_eq!(update.max_walk_speed, Some(600.0));
        assert_eq!(update.sprint_state_changed, Some(true));

        let update = state.fixed_tick(600.0, &SETTINGS);
        assert_eq!(state.meter, 0.5);
        assert_eq!(update.meter_percent, Some(0.5));
        assert_eq!(update.max_walk_speed, None);

        let update = state.fixed_tick(600.0, &SETTINGS);
        assert!(state.recovering);
        assert_eq!(update.max_walk_speed, Some(150.0));
        assert_eq!(update.meter_percent, Some(0.0));

        assert_eq!(state.end_sprint(&SETTINGS), SprintUpdate::default());
        assert!(!state.sprinting);
    }

    #[test]
    fn standing_still_does_not_burn_the_meter() {
        let (mut state, _) = SprintState::begin(&SETTINGS);
        state.start_sprint(&SETTINGS);
        let update = state.fixed_tick(100.0, &SETTINGS);
        assert_eq!(state.meter, 1.0);

        assert_eq!(update.max_walk_speed, Some(600.0));
        assert_eq!(update.sprint_state_changed, Some(true));
    }

    #[test]
    fn recovery_ends_when_the_meter_is_full_again() {
        let (mut state, _) = SprintState::begin(&SETTINGS);
        state.start_sprint(&SETTINGS);
        state.fixed_tick(600.0, &SETTINGS);
        state.fixed_tick(600.0, &SETTINGS);
        state.end_sprint(&SETTINGS);
        assert!(state.recovering);

        let update = state.fixed_tick(150.0, &SETTINGS);
        assert!(state.recovering);
        assert_eq!(update.max_walk_speed, None);

        let update = state.fixed_tick(150.0, &SETTINGS);
        assert!(!state.recovering);
        assert_eq!(update.max_walk_speed, Some(250.0));
        assert_eq!(update.sprint_state_changed, Some(false));
        assert_eq!(update.meter_percent, Some(1.0));
    }
}
