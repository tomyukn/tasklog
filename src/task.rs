use crate::parser::{parse_date, parse_time_hm};
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use chrono::Duration;
use getset::{Getters, Setters};
use std::collections::HashMap;
use std::fmt;
use std::ops;

/// A trait for displaying a time data.
pub trait TimeDisplay {
    /// Show the time as `HH:MM` format.
    fn to_string_hhmm(&self) -> String;
}

impl TimeDisplay for Duration {
    fn to_string_hhmm(&self) -> String {
        let minutes = self.num_minutes();
        let quo = (minutes / 60).abs();
        let rem = (minutes % 60).abs();
        let sign = if minutes < 0 { "-" } else { "" };
        format!("{}{:>02}:{:>02}", sign, quo, rem)
    }
}

/// A task represents a task log.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Getters, Setters)]
pub struct Task {
    #[getset(get = "pub")]
    id: Option<u32>,
    #[getset(get = "pub", set = "pub")]
    name: String,
    #[getset(get = "pub", set = "pub")]
    start_time: TaskTime,
    #[getset(get = "pub", set = "pub")]
    end_time: Option<TaskTime>,
    #[getset(get = "pub", set = "pub")]
    is_break_time: bool,
}

impl Task {
    /// Create a new task.
    pub fn new(
        id: Option<u32>,
        name: String,
        start_time: TaskTime,
        end_time: Option<TaskTime>,
        is_break_time: bool,
    ) -> Self {
        Self {
            id,
            name,
            start_time,
            end_time,
            is_break_time,
        }
    }

    /// Start a new task.
    pub fn start(name: String, time: TaskTime, is_break_time: bool) -> Self {
        Self::new(None, name, time, None, is_break_time)
    }

    /// End the task.
    pub fn end(self, time: TaskTime) -> Result<Self> {
        if time < self.start_time {
            return Err(anyhow!("end time is not after the start time"));
        }
        Ok(Self {
            end_time: Some(time),
            ..self
        })
    }

    /// Get the working date of the task.
    pub fn working_date(&self) -> WorkDate {
        WorkDate::from(self.start_time)
    }

    /// Calculate the duration of the task.
    fn duration(&self) -> Option<Duration> {
        self.end_time.map(|t| &t - &self.start_time)
    }

    pub fn duration_hhmm(&self) -> String {
        match &self.duration() {
            Some(duration) => duration.to_string_hhmm(),
            None => String::from(""),
        }
    }
}

/// A collection of tasks.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TaskList {
    tasks: Vec<(u32, Task)>,
}

impl TaskList {
    /// Create a `TaskList` from a vec of tasks
    pub fn new(tasks_with_seq: Vec<(u32, Task)>) -> Self {
        Self {
            tasks: tasks_with_seq,
        }
    }

    /// Return the summary of tasks
    pub fn summary(&self) -> Option<TaskSummary> {
        let tasks = self.tasks.clone();

        if tasks.is_empty() {
            return None;
        }

        let start_times = tasks
            .iter()
            .map(|(_, task)| task.start_time().clone())
            .collect::<Vec<_>>();

        // use start time if the end time is missing
        let end_times = tasks
            .iter()
            .map(|(_, task)| task.end_time().unwrap_or(*task.start_time()))
            .collect::<Vec<_>>();

        // overall start and end
        let start_first = start_times.clone().into_iter().min().unwrap();
        let end_last = end_times.clone().into_iter().max().unwrap();
        let duration_total = tasks
            .iter()
            .filter(|(_, task)| !task.is_break_time && task.duration().is_some())
            .fold(Duration::seconds(0), |acc, (_, task)| {
                acc + task.duration().unwrap()
            });

        // separate tasks to working and break
        let mut tasks_working = Vec::new();
        let mut tasks_break = Vec::new();
        for (_, task) in tasks {
            if task.is_break_time {
                tasks_break.push(task);
            } else {
                tasks_working.push(task);
            }
        }

        // sum durations by same tasks (without break times)
        let mut task_names_uniq = tasks_working
            .iter()
            .map(|task| task.name())
            .collect::<Vec<_>>();
        task_names_uniq.sort();
        task_names_uniq.dedup();

        let mut durations_map: HashMap<String, Duration> = HashMap::new();
        for name in task_names_uniq {
            durations_map.insert(name.to_string(), Duration::seconds(0));
        }

        for (name, duration) in tasks_working
            .iter()
            .map(|task| (task.name(), task.duration().unwrap_or(Duration::seconds(0))))
        {
            let duration_acc = durations_map.get(name).unwrap().clone();
            durations_map.insert(name.to_string(), duration_acc + duration);
        }

        Some(TaskSummary {
            start_time: start_first,
            end_time: end_last,
            duration_total,
            duration_by_taskname: durations_map,
            break_times: tasks_break,
        })
    }
}

impl IntoIterator for TaskList {
    type Item = (u32, Task);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.tasks.into_iter()
    }
}

/// A summary of tasks.
#[derive(Clone, PartialEq, Eq, Debug, Getters)]
pub struct TaskSummary {
    #[getset(get = "pub")]
    start_time: TaskTime,
    #[getset(get = "pub")]
    end_time: TaskTime,
    #[getset(get = "pub")]
    duration_total: Duration,
    #[getset(get = "pub")]
    duration_by_taskname: HashMap<String, Duration>,
    #[getset(get = "pub")]
    break_times: Vec<Task>,
}

/// A *date* for tasks which are considered belonging to the same day.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WorkDate(NaiveDate);

impl WorkDate {
    /// Create a `WorkDate` from invocation datetime.
    pub fn now() -> Self {
        Self::from(TaskTime::now())
    }

    /// Create a `WorkDate` from string.
    pub fn parse_from_str(s: &str) -> Result<Self> {
        let (y, m, d) = parse_date(&s)?;
        let date = NaiveDate::from_ymd_opt(y, m, d).ok_or(anyhow!("invalid date"))?;
        Ok(WorkDate(date))
    }
}

impl fmt::Display for WorkDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d").to_string())
    }
}

impl From<TaskTime> for WorkDate {
    fn from(tasktime: TaskTime) -> Self {
        let today = tasktime.0.date();
        let start = &today.and_hms(5, 0, 0);
        if &tasktime.0 >= &start {
            WorkDate(today)
        } else {
            WorkDate(today.pred())
        }
    }
}

/// A time representation for `Task`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TaskTime(NaiveDateTime);

impl TaskTime {
    ///
    pub fn now() -> Self {
        let now = Local::now().naive_local();
        TaskTime::from(now)
    }

    /// Create a `TaskTime` from hours and minutes.
    fn from_hm(hour: u32, min: u32) -> Self {
        let today = Local::today().naive_local();
        TaskTime(today.and_hms(hour, min, 0))
    }

    /// Create a `TaskTime` from a ISO8601 datetime format.
    pub fn parse_from_str_iso8601(s: &str) -> Result<Self> {
        match NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            Ok(t) => Ok(TaskTime(t.with_second(0).unwrap())),
            Err(e) => Err(anyhow!(e)),
        }
    }

    /// Create a `TaskTime` from a `"HHMM"` or `"HH:MM"` style string.
    pub fn parse_from_str_hhmm(s: &str) -> Result<Self> {
        let (hour, min) = parse_time_hm(s)?;
        Ok(TaskTime::from_hm(hour, min))
    }
}

impl fmt::Display for TaskTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%dT%H:%M:%S").to_string())
    }
}

impl From<NaiveDateTime> for TaskTime {
    fn from(datetime: NaiveDateTime) -> Self {
        TaskTime(datetime.with_second(0).unwrap())
    }
}

impl<'a, 'b> ops::Sub<&'a TaskTime> for &'b TaskTime {
    type Output = Duration;

    fn sub(self, other: &'a TaskTime) -> Duration {
        self.0 - other.0
    }
}

impl ops::Sub<TaskTime> for TaskTime {
    type Output = Duration;

    fn sub(self, other: TaskTime) -> Duration {
        self.0 - other.0
    }
}

impl TimeDisplay for TaskTime {
    fn to_string_hhmm(&self) -> String {
        self.0.format("%H:%M").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tasktime_to_string_hhmm() {
        let t = TaskTime(NaiveDate::from_ymd(2015, 9, 18).and_hms(23, 56, 0));
        assert_eq!(t.to_string_hhmm(), String::from("23:56"))
    }

    #[test]
    fn test_task_start() {
        let start_time = TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(11, 6, 0));
        let task = Task::start(String::from("task a"), start_time, false);
        assert_eq!(
            task,
            Task {
                id: None,
                name: String::from("task a"),
                start_time: start_time,
                end_time: None,
                is_break_time: false
            },
        );
    }

    #[test]
    fn test_task_end() {
        let start_time = TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(11, 6, 0));
        let task = Task {
            id: None,
            name: String::from("task a"),
            start_time: start_time,
            end_time: None,
            is_break_time: false,
        };

        let end_time1 = TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(11, 30, 0));
        assert_eq!(
            task.clone().end(end_time1).unwrap(),
            Task {
                id: None,
                name: String::from("task a"),
                start_time: start_time,
                end_time: Some(end_time1),
                is_break_time: false
            },
        );

        let end_time2 = TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(10, 30, 59));
        assert!(task.end(end_time2).is_err())
    }

    #[test]
    fn test_tasktime_duration() {
        let t1 = TaskTime(NaiveDate::from_ymd(2015, 9, 18).and_hms(23, 56, 0));
        let t2 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(1, 10, 0));

        let dur1 = &t2 - &t1;
        assert_eq!(dur1, chrono::Duration::minutes(74));
        assert_eq!(dur1.to_string_hhmm(), String::from("01:14"));

        let dur2 = &t1 - &t2;
        assert_eq!(dur2, chrono::Duration::minutes(-74));
        assert_eq!(dur2.to_string_hhmm(), String::from("-01:14"));
    }

    #[test]
    fn test_tasklist_summary() {
        let s1 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 0, 0));
        let e1 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 30, 0));
        let s2 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 30, 0));
        let e2 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 40, 0));
        let s3 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 40, 0));
        let e3 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 55, 0));

        let task1 = Task::start(String::from("task a"), s1, false)
            .end(e1)
            .unwrap();
        let task2 = Task::start(String::from("task b"), s2, false)
            .end(e2)
            .unwrap();
        let task3 = Task::start(String::from("task a"), s3, false)
            .end(e3)
            .unwrap();

        let tasklist = TaskList::new(vec![]);
        assert!(tasklist.summary().is_none());

        let tasklist = TaskList::new(vec![(0, task1), (1, task2), (2, task3)]);

        let mut duration_map = HashMap::new();
        duration_map.insert(String::from("task a"), Duration::minutes(45));
        duration_map.insert(String::from("task b"), Duration::minutes(10));

        assert_eq!(
            tasklist.summary(),
            Some(TaskSummary {
                start_time: s1,
                end_time: e3,
                duration_total: Duration::minutes(55),
                duration_by_taskname: duration_map,
                break_times: vec![]
            })
        );
    }

    #[test]
    fn test_tasklist_summary_with_break_time() {
        let s1 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 0, 0));
        let e1 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 30, 0));
        let s2 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 30, 0));
        let e2 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 40, 0));
        let s3 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 40, 0));
        let e3 = TaskTime(NaiveDate::from_ymd(2015, 9, 19).and_hms(10, 55, 0));

        let task1 = Task::start(String::from("task a"), s1, true)
            .end(e1)
            .unwrap();
        let task2 = Task::start(String::from("task b"), s2, false)
            .end(e2)
            .unwrap();
        let task3 = Task::start(String::from("task c"), s3, true)
            .end(e3)
            .unwrap();

        let tasklist = TaskList::new(vec![]);
        assert!(tasklist.summary().is_none());

        let tasklist = TaskList::new(vec![(0, task1.clone()), (1, task2), (2, task3.clone())]);

        let mut duration_map = HashMap::new();
        duration_map.insert(String::from("task b"), Duration::minutes(10));

        assert_eq!(
            tasklist.summary(),
            Some(TaskSummary {
                start_time: s1,
                end_time: e3,
                duration_total: Duration::minutes(10),
                duration_by_taskname: duration_map,
                break_times: vec![task1, task3]
            })
        );
    }

    #[test]
    fn test_workdate_creation_from_tasktime() {
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(5, 0, 0))),
            WorkDate::parse_from_str("2021-01-01").unwrap()
        );
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(23, 59, 0))),
            WorkDate::parse_from_str("2021-01-01").unwrap()
        );
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(0, 0, 0))),
            WorkDate::parse_from_str("2021-01-01").unwrap()
        );
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(4, 59, 0))),
            WorkDate::parse_from_str("2021-01-01").unwrap()
        );
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(5, 0, 0))),
            WorkDate::parse_from_str("2021-01-02").unwrap()
        );
    }

    #[test]
    fn test_workdate_to_string() {
        assert_eq!(
            WorkDate::parse_from_str("2021-01-01").unwrap().to_string(),
            String::from("2021-01-01")
        );
        assert_eq!(
            WorkDate::parse_from_str("20210101").unwrap().to_string(),
            String::from("2021-01-01")
        );
    }

    #[test]
    fn test_tasktime_from_string() {
        assert_eq!(
            TaskTime::parse_from_str_iso8601("2021-01-01T12:34:56").unwrap(),
            TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(12, 34, 0))
        );
    }

    #[test]
    fn test_tasktime_from_datetime() {
        assert_eq!(
            TaskTime::from(NaiveDate::from_ymd(2021, 1, 1).and_hms(12, 34, 56)),
            TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(12, 34, 0))
        );
    }

    #[test]
    fn test_tasktime_to_string() {
        assert_eq!(
            TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(12, 34, 0)).to_string(),
            "2021-01-01T12:34:00"
        );
    }

    #[test]
    fn test_tasktime_subtractions() {
        let t1 = TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(12, 30, 0));
        let t2 = TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(12, 45, 0));
        assert_eq!(&t2 - &t1, Duration::minutes(15));
        assert_eq!(t2 - t1, Duration::minutes(15));
    }
}
