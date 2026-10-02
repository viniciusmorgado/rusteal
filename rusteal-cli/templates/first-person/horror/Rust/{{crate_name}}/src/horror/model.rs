// Pure logic for the horror character's stamina-based sprint: no engine calls,
// fully unit-tested. `AHorrorCharacter` keeps it in a few fields and runs it
// from its input and a fixed-interval timer; here it is one state machine.

/// The character's sprint tuning, its editable properties.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SprintSettings {
    /// Default walk speed when not sprinting or recovering
    pub walk_speed: f32,
    /// Time interval for sprinting stamina ticks
    pub fixed_tick_time: f32,
    /// How long we can sprint for, in seconds
    pub sprint_time: f32,
    /// Walk speed while sprinting
    pub sprint_speed: f32,
    /// Walk speed while recovering stamina
    pub recovering_walk_speed: f32,
}

/// What the character does after a sprint event: set its max walk speed,
/// tell its UI the sprint state changed, tell its UI the meter changed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SprintUpdate {
    pub max_walk_speed: Option<f32>,
    pub sprint_state_changed: Option<bool>,
    pub meter_percent: Option<f32>,
}

/// The sprint flags and the stamina meter.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SprintState {
    /// If true, we're sprinting
    pub sprinting: bool,
    /// If true, we're recovering stamina
    pub recovering: bool,
    /// Sprint stamina amount. Maxes at the sprint time
    pub meter: f32,
}

impl SprintState {
    /// `BeginPlay`: a full meter, walking.
    pub fn begin(settings: &SprintSettings) -> (Self, SprintUpdate) {
        let state = SprintState {
            // initialize sprint meter to max
            meter: settings.sprint_time,
            ..Default::default()
        };
        // Initialize the walk speed
        let update = SprintUpdate {
            max_walk_speed: Some(settings.walk_speed),
            ..Default::default()
        };
        (state, update)
    }

    /// `DoStartSprint`.
    pub fn start_sprint(&mut self, settings: &SprintSettings) -> SprintUpdate {
        self.set_sprinting(true, settings.sprint_speed)
    }

    /// `DoEndSprint`.
    pub fn end_sprint(&mut self, settings: &SprintSettings) -> SprintUpdate {
        self.set_sprinting(false, settings.walk_speed)
    }

    fn set_sprinting(&mut self, sprinting: bool, speed: f32) -> SprintUpdate {
        // set the sprinting flag
        self.sprinting = sprinting;

        // are we out of recovery mode?
        if self.recovering {
            return SprintUpdate::default();
        }
        // set the walk speed and call the sprint state changed delegate
        SprintUpdate {
            max_walk_speed: Some(speed),
            sprint_state_changed: Some(sprinting),
            meter_percent: None,
        }
    }

    /// `SprintFixedTick`, called while playing at a fixed time interval, with
    /// the character's current speed.
    pub fn fixed_tick(&mut self, speed: f64, settings: &SprintSettings) -> SprintUpdate {
        let mut update = SprintUpdate::default();

        // are we out of recovery, still have stamina and are moving faster than our walk speed?
        if self.sprinting && !self.recovering && speed > f64::from(settings.walk_speed) {
            // do we still have meter to burn?
            if self.meter > 0.0 {
                // update the sprint meter
                self.meter = (self.meter - settings.fixed_tick_time).max(0.0);

                // have we run out of stamina?
                if self.meter <= 0.0 {
                    // raise the recovering flag
                    self.recovering = true;

                    // set the recovering walk speed
                    update.max_walk_speed = Some(settings.recovering_walk_speed);
                }
            }
        } else {
            // recover stamina
            self.meter = (self.meter + settings.fixed_tick_time).min(settings.sprint_time);

            if self.meter >= settings.sprint_time {
                // lower the recovering flag
                self.recovering = false;

                // set the walk or sprint speed depending on whether the sprint button is down
                update.max_walk_speed = Some(if self.sprinting {
                    settings.sprint_speed
                } else {
                    settings.walk_speed
                });

                // update the sprint state depending on whether the button is down or not
                update.sprint_state_changed = Some(self.sprinting);
            }
        }

        // broadcast the sprint meter updated delegate
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

        // releasing the button while recovering changes nothing but the flag
        assert_eq!(state.end_sprint(&SETTINGS), SprintUpdate::default());
        assert!(!state.sprinting);
    }

    #[test]
    fn standing_still_does_not_burn_the_meter() {
        let (mut state, _) = SprintState::begin(&SETTINGS);
        state.start_sprint(&SETTINGS);
        let update = state.fixed_tick(100.0, &SETTINGS);
        assert_eq!(state.meter, 1.0);
        // a full meter ends any recovery and restates the speed
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
