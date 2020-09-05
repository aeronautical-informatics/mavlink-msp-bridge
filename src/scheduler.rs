use smol::lock::Mutex;
use std::cmp::Eq;
use std::convert::TryInto;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use smol::Timer;

/// `std::Vec` representing one major timeframe of `duration` length,
/// divided in `size` slots of which each may contain one MAVLink message id.
pub struct Schedule<T: Clone + Copy + PartialEq> {
    time: Vec<ArcSwapOption<T>>,
    len: u32,
    duration: Duration,
    frame: Arc<Mutex<FrameInformation>>,
}

#[derive(Clone)]
struct FrameInformation {
    last: u128,
    last_time: Instant,
}

impl<T: Clone + Copy + PartialEq> Schedule<T> {
    /// Initializes a new instance of `Schedule`
    pub fn new(size: usize) -> Self {
        Schedule {
            time: vec![ArcSwapOption::from(None); size],
            len: size.try_into().expect("Schedule too big"),
            duration: Duration::new(1, 0),
            frame: Arc::new(Mutex::new(FrameInformation {
                last: 0,
                last_time: Instant::now(),
            })),
        }
    }

    /// yields the next event of the schedule
    pub async fn next(&self) -> T {
        loop {
            let mut fi = self.frame.lock().await;
            let index = (fi.last % self.len as u128) as usize;
            let minor_frame_duration = self.duration / self.len;
            let next_minor_frame_time = fi.last_time + minor_frame_duration;

            Timer::at(next_minor_frame_time).await;
            fi.last_time = next_minor_frame_time;
            fi.last += 1;
            if let Some(task) = &*self.time[index].load() {
                return **task;
            }
        }
    }

    /// counts the occurences of a given task in the current schedule
    pub fn count(&self, task: &T) -> usize {
        self.time
            .iter()
            .filter(|mt| matches!(mt.load().as_ref(), Some(ref t) if *task == ***t))
            .count()
    }

    /// tries to insert a schedule into
    pub fn insert(&self, frequency: u32, task: T) -> Result<(), &'static str> {
        if frequency == 0 {
            self.delete(&task);
            return Ok(());
        }
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
            .map(|mt| if (*mt.load()).is_none() { 0 } else { 1 })
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
                        assert!(self.time[i].load().is_none());
                        self.time[i].store(Some(Arc::new(task)));
                    }
                }
                Ok(())
            }
            _ => Err("task does not fit current schedule"),
        }
    }

    pub fn delete(&self, task: &T) {
        self.time.iter().for_each(|mt| match mt.load().as_ref() {
            Some(ref t) if *task == ***t => mt.store(None),
            _ => {}
        })
    }
}

impl<T: Copy + Eq + ToString> fmt::Display for Schedule<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Schedule {{size: {}, duration: {:#?} }}:",
            self.time.len(),
            self.duration
        )?;
        write!(
            f,
            "{}",
            self.time
                .iter()
                .map(|mt| match mt.load().as_ref() {
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
    use smol;
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Task {
        id: u32,
    }

    #[test]
    fn simple() {
        let s = Schedule::new(200);
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
        let freq = 7;
        let s = Schedule::new(200);
        let range = 0..20;
        for i in range.clone() {
            let t = Task { id: i };
            s.insert(freq, t).unwrap();
        }
        for i in range {
            let count = s.count(&Task { id: i });
            assert_eq!(count, freq as usize);
        }
    }

    #[test]
    fn multiple_similar_frequencies() {
        let s = Schedule::new(200);
        for i in 3..14 {
            let t = Task { id: i };
            s.insert(i % 5 + 1, t).unwrap();
        }
    }

    // TODO: check actual timing
    #[test]
    fn timing_behaviour() {
        let s = Schedule::new(10);
        let t = Task { id: 1 };
        let t0 = Instant::now();
        let tol = Duration::from_millis(10);
        let hundred_milli = Duration::from_millis(100);
        smol::block_on(async move {
            s.insert(3, t).unwrap();
            assert_eq!(s.next().await, t);
            assert!(hundred_milli < t0.elapsed() && t0.elapsed() < hundred_milli + tol);
            sleep(Duration::from_millis(700));
            assert_eq!(s.next().await, t);
            assert_eq!(s.next().await, t);
            sleep(Duration::from_millis(1000));
            assert_eq!(s.next().await, t);
            assert_eq!(s.next().await, t);
            assert_eq!(s.next().await, t);
        });
    }
}
