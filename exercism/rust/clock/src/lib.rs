use core::fmt::Display;
use std::fmt::Debug;

#[derive(Debug, Eq, PartialEq)]
pub struct Clock(i32, i32);

impl Clock {
    pub fn new(hours: i32, minutes: i32) -> Self {
        Clock(hours, minutes)
    }

    pub fn add_minutes(&self, minutes: i32) -> Self {
        Clock(self.0, self.1 + minutes)
    }
}

impl Display for Clock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.0, self.1)
    }
}
