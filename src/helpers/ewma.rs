//Implementation of https://en.wikipedia.org/wiki/Moving_average#Application_to_measuring_computer_performance
pub struct TimeWindowedEwma {
    current_estimation: f64,
    window_len: f64,
    last_event_time: f64
}

impl TimeWindowedEwma {
    pub fn new (window_len: f64) -> Self
    {
        TimeWindowedEwma {
            current_estimation: 0.,
            window_len,
            last_event_time: 0.
        }
    }

    pub fn from_initial_value (initial_value: f64, window_len: f64) -> Self
    {
        TimeWindowedEwma {
            current_estimation: initial_value,
            window_len,
            last_event_time: -1. //Indicates that we should trust the initial value
        }
    }

    pub fn update(&mut self, time: f64, value: f64) -> f64
    {
        let alpha = if self.last_event_time < 0. { 0.01 } // We trust the initial value a lot
                    else { 1. - (-(time - self.last_event_time) / self.window_len).exp() };
        self.current_estimation = (1.- alpha) * self.current_estimation + alpha * value;
        self.last_event_time = time;
        self.current_estimation
    }

    pub fn get_current_estimation (&self) -> f64
    {
        self.current_estimation
    }
}
