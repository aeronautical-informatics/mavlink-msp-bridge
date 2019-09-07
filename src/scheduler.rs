use std::fmt;
use std::time::Duration;

/// `std::Vec` representing one major timeframe of `duration` length,
/// divided in `size` slots of which each may contain one MAVLink message id.
pub struct Schedule {
    time: Vec<Option<Task>>,
    duration: Duration,
}

impl Schedule {
    pub fn new(size: usize) -> Self {
        Schedule {
            time: vec![None; size],
            duration: Duration::new(1, 0),
        }
    }

    pub fn count(&self, t: &Task) -> usize {
        self.time
            .iter()
            .filter_map(|a| *a)
            .filter(|a| a == t)
            .count()
    }

    /// tries to insert a schedule into
    pub fn insert(&mut self, frequency: u32, task: Task) -> Result<(), &'static str> {
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

    pub fn delete(&mut self, task: &Task) {
        for a in &mut self.time {
            match a {
                Some(t) if t == task => *a = None,
                _ => (),
            }
        }
    }
}

impl fmt::Display for Schedule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Schedule {{size: {}, duartion: {:#?} }}:\n",
            self.time.len(),
            self.duration
        )?;
        write!(
            f,
            "{}",
            self.time
                .iter()
                .map(|a| match a {
                    Some(task) => task.id.to_string(),
                    _ => "-".to_string(),
                })
                .fold(String::new(), |a, b| a + &b)
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Task {
    id: u32,
}

#[cfg(test)]
mod test {
    use super::*;

    /// Calculates the frequency of occurences of a certain task
    pub fn frequency(s: &Schedule, task: &Task) -> f64 {
        let count = s.count(task) as f64;
        count / s.duration.as_secs_f64()
    }

    /// Calculates the variance of the intervals in between the occurences of a certain task
    pub fn frequency_jitter(s: &Schedule, task: &Task) -> f64 {
        let indexes: Vec<usize> = s
            .time
            .iter()
            .enumerate()
            .filter_map(|(i, e)| match e {
                Some(t) if t == task => Some(i),
                _ => None,
            })
            .collect();
        let intervals = indexes
            .iter()
            .enumerate()
            .map(|(i, e)| (indexes[(i + 1) % indexes.len()] + s.time.len() - e) % s.time.len());
        let average: f64 = intervals.clone().sum::<usize>() as f64 / intervals.len() as f64;
        intervals
            .clone()
            .map(|a| (a as f64 - average).powf(2.))
            .sum::<f64>()
            / intervals.len() as f64
    }

    #[test]
    fn multiple_frequencies() {
        let mut tasks: Vec<(Task, u32)> = Vec::new();
        let mut s = Schedule::new(200);

        for i in 3..10 {
            let t = Task { id: i };
            tasks.push((t, i));
            s.insert(i, t).unwrap();
        }

        for (t, f) in tasks {
            assert_eq!(frequency(&s, &t), f as f64);
            assert!(frequency_jitter(&s, &t) < 1.0);
        }
    }

    #[test]
    fn multiple_identical_frequencies() {
        let mut tasks: Vec<(Task, u32)> = Vec::new();
        let mut s = Schedule::new(200);

        for i in 0..20 {
            let t = Task { id: i };
            tasks.push((t, 7));
            s.insert(7, t).unwrap();
        }

        for (t, f) in tasks {
            assert_eq!(frequency(&s, &t), f as f64);
            assert!(frequency_jitter(&s, &t) < 1.0);
        }
    }

    #[test]
    fn multiple_similar_frequencies() {
        let mut tasks: Vec<(Task, u32)> = Vec::new();
        let mut s = Schedule::new(200);

        for i in 3..14 {
            let t = Task { id: i };
            let f = i % 5 + 1;
            tasks.push((t, f));
            s.insert(f, t).unwrap();
        }

        for (t, f) in tasks {
            assert_eq!(frequency(&s, &t), f as f64);
            assert!(frequency_jitter(&s, &t) < 1.0);
        }
    }

    #[test]
    fn fuzzy_add_remove() {}

    #[test]
    fn frequency_stability_() {}
}
