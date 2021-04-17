use anyhow::{anyhow, Result};
use chrono::prelude::*;
use chrono::Duration;
use getset::{Getters, Setters};
use regex::Regex;
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
    #[getset(get = "pub")]
    working_date: WorkDate,
    #[getset(get = "pub", set = "pub")]
    start_time: TaskTime,
    #[getset(get = "pub", set = "pub")]
    end_time: Option<TaskTime>,
}

impl Task {
    /// Create a new task.
    pub fn new(
        id: Option<u32>,
        name: String,
        start_time: TaskTime,
        end_time: Option<TaskTime>,
    ) -> Self {
        let working_date = WorkDate::from(start_time);

        Self {
            id,
            name,
            working_date,
            start_time,
            end_time,
        }
    }

    /// Start a new task.
    pub fn start(name: String, time: TaskTime) -> Self {
        Self::new(None, name, time, None)
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

    /// Calculate the duration of the task.
    fn duration(&self) -> Option<Duration> {
        self.end_time.map(|t| &t - &self.start_time)
    }

    pub fn duration_hhmm(&self) -> String {
        if let Some(duration) = &self.duration() {
            duration.to_string_hhmm()
        } else {
            String::from("")
        }
    }
}

/// A collection of tasks.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TaskList {
    tasks: Vec<Task>,
}

impl TaskList {
    /// Create a `TaskList` from a vec of tasks
    pub fn new(tasks: Vec<Task>) -> Self {
        TaskList { tasks }
    }

    /// Return the summary of tasks
    pub fn summary(&self) -> Option<TaskSummary> {
        let tasks = self.tasks.clone();

        if tasks.is_empty() {
            return None;
        }

        let start_times = tasks
            .iter()
            .map(|task| task.start_time().clone())
            .collect::<Vec<_>>();

        // use start time if the end time is missing
        let end_times = tasks
            .iter()
            .map(|task| task.end_time().unwrap_or(*task.start_time()))
            .collect::<Vec<_>>();

        let start_first = start_times.clone().into_iter().min().unwrap();
        let end_last = end_times.clone().into_iter().max().unwrap();

        let task_names = tasks.iter().map(|task| task.name()).collect::<Vec<_>>();

        let durations = tasks
            .iter()
            .map(|task| task.duration().unwrap_or(Duration::seconds(0)))
            .collect::<Vec<_>>();

        let duration_total = tasks
            .iter()
            .filter(|task| task.duration().is_some())
            .fold(Duration::seconds(0), |acc, task| {
                acc + task.duration().unwrap()
            });

        // compute total duration by each task
        let mut durations_map: HashMap<String, Duration> = HashMap::new();

        let mut task_names_uniq = task_names.clone();
        task_names_uniq.sort();
        task_names_uniq.dedup();

        // placeholder
        for name in task_names_uniq {
            durations_map.insert(name.to_string(), Duration::seconds(0));
        }

        for (name, duration) in task_names.into_iter().zip(durations) {
            let duration_acc = durations_map.get(name).unwrap().clone();
            durations_map.insert(name.to_string(), duration_acc + duration);
        }

        Some(TaskSummary {
            start: start_first,
            end: end_last,
            duration_total,
            duration_by_taskname: durations_map,
        })
    }
}

/// A summary of tasks.
#[derive(Clone, PartialEq, Eq, Debug, Getters)]
pub struct TaskSummary {
    #[getset(get = "pub")]
    start: TaskTime,
    #[getset(get = "pub")]
    end: TaskTime,
    #[getset(get = "pub")]
    duration_total: Duration,
    #[getset(get = "pub")]
    duration_by_taskname: HashMap<String, Duration>,
}

/// A *date* for tasks which are considered belonging to the same day.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WorkDate(NaiveDate);

impl WorkDate {
    /// Create a `WorkDate` from invocation datetime.
    pub fn now() -> Self {
        Self::from(TaskTime::now())
    }
}

impl fmt::Display for WorkDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d").to_string())
    }
}

impl From<String> for WorkDate {
    fn from(date: String) -> Self {
        let d = NaiveDate::parse_from_str(&date, "%Y-%m-%d").expect("invalid date format");
        WorkDate(d)
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
    pub fn parse_from_string_iso8601(s: String) -> Result<Self> {
        match NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
            Ok(t) => Ok(TaskTime(t.with_second(0).unwrap())),
            Err(e) => Err(anyhow!(e)),
        }
    }

    /// Create a `TaskTime` from a `"HHMM"` or `"HH:MM"` style string.
    pub fn from_string_hhmm(hhmm: String) -> Result<Self> {
        let (hour, min) = parse_hhmm(hhmm)?;
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

// Parse an `"HHMM"` or `"HH:MM"` style string to a tuple of int values which replesents
// hours and minutes.
fn parse_hhmm(hhmm: String) -> Result<(u32, u32)> {
    let re = Regex::new(r"^([0-2][0-9]|[0-9]):?([0-5][0-9])$").unwrap();
    let captures = re.captures(&hhmm).ok_or(anyhow!("invalid time format"))?;
    let h = captures.get(1).unwrap().as_str().parse::<u32>()?;
    let m = captures.get(2).unwrap().as_str().parse::<u32>()?;
    if h < 24 && m < 60 {
        Ok((h, m))
    } else {
        Err(anyhow!("invalid time range"))
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
        let task = Task::start(String::from("task a"), start_time);
        assert_eq!(
            task,
            Task {
                id: None,
                name: String::from("task a"),
                working_date: WorkDate::from(String::from("2021-01-02")),
                start_time: start_time,
                end_time: None
            },
        );
    }

    #[test]
    fn test_task_end() {
        let start_time = TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(11, 6, 0));
        let task = Task {
            id: None,
            name: String::from("task a"),
            working_date: WorkDate::from(String::from("2021-01-02")),
            start_time: start_time,
            end_time: None,
        };

        let end_time1 = TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(11, 30, 0));
        assert_eq!(
            task.clone().end(end_time1).unwrap(),
            Task {
                id: None,
                name: String::from("task a"),
                working_date: WorkDate::from(String::from("2021-01-02")),
                start_time: start_time,
                end_time: Some(end_time1)
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

        let task1 = Task::start(String::from("task a"), s1).end(e1).unwrap();
        let task2 = Task::start(String::from("task b"), s2).end(e2).unwrap();
        let task3 = Task::start(String::from("task a"), s3).end(e3).unwrap();

        let tasklist = TaskList::new(vec![]);
        assert!(tasklist.summary().is_none());

        let tasklist = TaskList::new(vec![task1, task2, task3]);

        let mut duration_map = HashMap::new();
        duration_map.insert(String::from("task a"), Duration::minutes(45));
        duration_map.insert(String::from("task b"), Duration::minutes(10));

        assert_eq!(
            tasklist.summary(),
            Some(TaskSummary {
                start: s1,
                end: e3,
                duration_total: e3 - s1,
                duration_by_taskname: duration_map
            })
        );
    }

    #[test]
    fn test_workdate_creation_from_tasktime() {
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(5, 0, 0))),
            WorkDate::from(String::from("2021-01-01"))
        );
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 1).and_hms(23, 59, 0))),
            WorkDate::from(String::from("2021-01-01"))
        );
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(0, 0, 0))),
            WorkDate::from(String::from("2021-01-01"))
        );
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(4, 59, 0))),
            WorkDate::from(String::from("2021-01-01"))
        );
        assert_eq!(
            WorkDate::from(TaskTime(NaiveDate::from_ymd(2021, 1, 2).and_hms(5, 0, 0))),
            WorkDate::from(String::from("2021-01-02"))
        );
    }

    #[test]
    fn test_workdate_to_string() {
        assert_eq!(
            WorkDate::from(String::from("2021-01-01")).to_string(),
            String::from("2021-01-01")
        );
    }

    #[test]
    fn test_tasktime_from_string() {
        assert_eq!(
            TaskTime::parse_from_string_iso8601(String::from("2021-01-01T12:34:56")).unwrap(),
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

    #[test]
    fn test_parse_hhmm() {
        assert_eq!(parse_hhmm(String::from("2310")).unwrap(), (23, 10));
        assert_eq!(parse_hhmm(String::from("0559")).unwrap(), (5, 59));
        assert_eq!(parse_hhmm(String::from("0605")).unwrap(), (6, 5));
        assert_eq!(parse_hhmm(String::from("23:10")).unwrap(), (23, 10));
        assert_eq!(parse_hhmm(String::from("05:59")).unwrap(), (5, 59));
        assert_eq!(parse_hhmm(String::from("6:05")).unwrap(), (6, 5));

        assert!(parse_hhmm(String::from("aaa")).is_err());
        assert!(parse_hhmm(String::from("2410")).is_err());
        assert!(parse_hhmm(String::from("0560")).is_err());
        assert!(parse_hhmm(String::from("24:10")).is_err());
        assert!(parse_hhmm(String::from("05:60")).is_err());
        assert!(parse_hhmm(String::from("5:60")).is_err());
    }
}
