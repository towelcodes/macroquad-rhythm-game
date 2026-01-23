use std::{
    ops::{Add, Div, Mul, Sub},
    time::{Duration, Instant},
};

#[derive(PartialEq, Debug)]
enum TweenState {
    NotStarted,
    Playing,
    Finished,
}

#[derive(PartialEq, Debug)]
pub enum TweenEasing {
    Linear,
    EaseInOut,
    EaseOut,
    EaseIn,
}

/// An animation playing over time
/// TODO: Sync to audio clock
pub struct Tween<T> {
    value: T,
    target: T,
    start: Option<Instant>,
    end: Instant,
    state: TweenState,
    easing: TweenEasing,
}
impl<T> Tween<T>
where
    T: Sub<Output = T>
        + Add<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + From<f32>
        + Copy
        + PartialOrd,
{
    pub fn new(value: T, target: T, duration: Duration, easing: TweenEasing) -> Self {
        Self {
            value,
            target,
            easing,
            start: None,
            end: Instant::now()
                .checked_add(duration)
                .expect("Failed to calculate end time"),
            state: TweenState::NotStarted,
        }
    }

    #[allow(dead_code)]
    pub fn state(&self) -> &TweenState {
        &self.state
    }

    /// Get the current value
    pub fn get(&mut self) -> T {
        if self.state == TweenState::Finished {
            return self.target;
        }
        if self.start.is_none() {
            self.start = Some(Instant::now());
            self.state = TweenState::Playing;
        }
        let start = self.start.unwrap();
        let multiplier = (start.elapsed().as_millis() as f32)
            / (self.end.duration_since(start).as_millis() as f32);
        let easing_multiplier = match self.easing {
            TweenEasing::Linear => multiplier,
            TweenEasing::EaseInOut => {
                if multiplier < 0.5 {
                    2f32 * multiplier * multiplier
                } else {
                    -1f32 + (4f32 - 2f32 * multiplier) * multiplier
                }
            }
            TweenEasing::EaseOut => {
                let t = multiplier - 1f32;
                1f32 + t * t * t
            }
            TweenEasing::EaseIn => multiplier * multiplier * multiplier,
        };

        if multiplier > 1f32 {
            self.state = TweenState::Finished;
            return self.target;
        }

        self.value + ((self.target - self.value) * T::from(easing_multiplier))
    }
}
