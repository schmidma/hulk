use std::{
    f32::consts::{PI, TAU},
    ops::{Add, Mul, Sub},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct LowPassFilter<State> {
    smoothing_factor: f32,
    state: State,
}

impl<State> LowPassFilter<State>
where
    State: Copy + Add<Output = State> + Sub<Output = State> + Mul<f32, Output = State>,
{
    pub fn with_smoothing_factor(initial_state: State, smoothing_factor: f32) -> Self {
        Self {
            smoothing_factor,
            state: initial_state,
        }
    }

    pub fn with_cutoff(initial_state: State, cutoff_frequency: f32, sampling_rate: f32) -> Self {
        let rc = 1.0 / (cutoff_frequency * TAU);
        let dt = 1.0 / sampling_rate;
        let smoothing_factor = dt / (rc + dt);
        Self {
            smoothing_factor,
            state: initial_state,
        }
    }

    pub fn update(&mut self, value: State) {
        self.state = self.state + (value - self.state) * self.smoothing_factor;
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn reset(&mut self, state: State) {
        self.state = state;
    }
}
