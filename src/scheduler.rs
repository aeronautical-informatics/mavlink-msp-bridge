use std::cmp::Eq;
use std::fmt;
use std::time::{Duration, Instant};

/// `std::Vec` representing one major timeframe of `duration` length,
/// divided in `size` slots of which each may contain one MAVLink message id.
pub struct Schedule<T: Clone + Copy + PartialEq> {
    time: Vec<Option<T>>,
    duration: Duration,
    last_frame: u128,
    time_zero: Instant,
}

impl<T: Clone + Copy + PartialEq> Schedule<T> {
    pub fn new(size: usize) -> Self {
        Schedule {
            time: vec![None; size],
            duration: Duration::new(1, 0),
            last_frame: 0,
            time_zero: Instant::now(),
        }
    }

    pub fn next(&mut self) -> Option<T> {
        let duration = self.duration.as_nanos();
        let elapsed = self.time_zero.elapsed().as_nanos();
        let current_frame_index = elapsed * self.time.len() as u128 / duration;

        if self.last_frame == current_frame_index {
            return None;
        }
        assert!(self.last_frame < current_frame_index);

        self.last_frame += 1;
        let mut index = (self.last_frame % self.time.len() as u128) as usize;
        while self.time[index].is_none() && self.last_frame < current_frame_index {
            self.last_frame += 1;
            index = (self.last_frame % self.time.len() as u128) as usize;
        }
        self.time[index]
    }

    /// counts the occurences of a given task in the current schedule
    pub fn count(&self, task: &T) -> usize {
        self.time
            .iter()
            .filter_map(|t| *t)
            .filter(|t| t == task)
            .count()
    }

    /// tries to insert a schedule into
    pub fn insert(&mut self, frequency: u32, task: T) -> Result<(), &'static str> {
        let mut new_schedule = vec![0; self.time.len()];
        let interval = self.time.len() as f64 / frequency as f64 / self.duration.as_secs_f64();

        let frame_count = (self.duration.as_secs_f64() * frequency as f64).round() as usize;
        for i in 0..frame_count {
            let index = (i as f64 * interval).round() as usize;
            new_schedule[index] = 1;
        }

        let mut min = usize::max_value();
        let mut tau = 0;
        let time_use: Vec<usize> = self
            .time
            .iter()
            .map(|a| if a.is_none() { 0 } else { 1 })
            .collect();
        for i in 0..self.time.len() {
            let sum: usize = time_use
                .iter()
                .zip(new_schedule.iter().cycle().skip(i))
                .map(|a| a.0 * a.1)
                .sum();
            if sum < min {
                min = sum;
                tau = i;
            }
            if sum == 0 {
                break;
            }
        }
        match min {
            0 => {
                for (i, t) in new_schedule
                    .iter()
                    .cycle()
                    .skip(tau)
                    .enumerate()
                    .take(self.time.len())
                {
                    if *t == 1 {
                        assert!(self.time[i].is_none());
                        self.time[i] = Some(task);
                    }
                }
                Ok(())
            }
            _ => Err("task does not fit current schedule"),
        }
    }

    pub fn delete(&mut self, task: &T) {
        for i in 0..self.time.len() {
            match self.time[i] {
                Some(t) if t == *task => self.time[i] = None,
                _ => (),
            }
        }
    }
}

impl<T: Copy + Eq + ToString> fmt::Display for Schedule<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Schedule {{size: {}, duration: {:#?} }}:\n",
            self.time.len(),
            self.duration
        )?;
        write!(
            f,
            "{}",
            self.time
                .iter()
                .map(|a| match a {
                    Some(task) => task.to_string(),
                    _ => "-".to_string(),
                })
                .fold(String::new(), |a, b| a + &b)
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::thread;

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Task {
        id: u32,
    }

    #[test]
    fn simple() {
        let mut s = Schedule::new(200);
        let range = 3..10;
        for i in range.clone() {
            let t = Task { id: i };
            s.insert(i, t).unwrap();
        }
        for i in range {
            let count = s.count(&Task { id: i });
            assert_eq!(count, i as usize);
        }
    }

    #[test]
    fn multiple_identical_frequencies() {
        let FREQ = 7;
        let mut s = Schedule::new(200);
        let range = 0..20;
        for i in range.clone() {
            let t = Task { id: i };
            s.insert(FREQ, t).unwrap();
        }
        for i in range {
            let count = s.count(&Task { id: i });
            assert_eq!(count, FREQ as usize);
        }
    }

    #[test]
    fn multiple_similar_frequencies() {
        let mut s = Schedule::new(200);
        for i in 3..14 {
            let t = Task { id: i };
            s.insert(i % 5 + 1, t).unwrap();
        }
    }

    #[test]
    fn timing_behaviour() {
        let mut s = Schedule::new(10);
        let t = Task { id: 1 };
        assert_eq!(s.next(), None);
        s.insert(3, t).unwrap();
        assert_eq!(s.next(), None);
        thread::sleep(Duration::from_millis(100));
        assert_eq!(s.next(), None);
        assert_eq!(s.next(), None);
        assert_eq!(s.next(), None);
        thread::sleep(Duration::from_millis(300));
        assert_eq!(s.next(), Some(Task { id: 1 }));
        assert_eq!(s.next(), None);
        assert_eq!(s.next(), None);
        assert_eq!(s.next(), None);
        thread::sleep(Duration::from_millis(700));
        assert_eq!(s.next(), Some(Task { id: 1 }));
        assert_eq!(s.next(), Some(Task { id: 1 }));
        assert_eq!(s.next(), None);
        assert_eq!(s.next(), None);
        assert_eq!(s.next(), None);
        thread::sleep(Duration::from_millis(1000));
        assert_eq!(s.next(), Some(Task { id: 1 }));
        assert_eq!(s.next(), Some(Task { id: 1 }));
        assert_eq!(s.next(), Some(Task { id: 1 }));
        assert_eq!(s.next(), None);
    }
}
