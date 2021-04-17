use crate::task::{Task, TaskTime, WorkDate};
use anyhow::{anyhow, Result};
use getset::Getters;
use rusqlite::{params, Connection, OpenFlags, NO_PARAMS};
use std::env;
use std::fmt;
use std::path::PathBuf;

/// A Struct represents a database.
#[derive(Getters)]
pub struct Database {
    conn: Connection,
    #[getset(get = "pub")]
    location: DatabaseLocation,
}

impl Database {
    pub fn path(&self) -> Option<PathBuf> {
        match &self.location {
            DatabaseLocation::Memory => None,
            DatabaseLocation::File(path) => Some(path.to_path_buf()),
        }
    }

    /// Connect to the database.
    fn connect(path: &PathBuf, flags: OpenFlags) -> Result<Database> {
        let conn = Connection::open_with_flags(path, flags)?;

        Ok(Database {
            conn,
            location: DatabaseLocation::File(path.to_path_buf()),
        })
    }

    /// Connect to the database (read/write/create mode).
    pub fn connect_rwc(path: &PathBuf) -> Result<Database> {
        Self::connect(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
    }

    /// Connect to the database (read/write mode).
    pub fn connect_rw(path: &PathBuf) -> Result<Database> {
        Self::connect(path, OpenFlags::SQLITE_OPEN_READ_WRITE)
    }

    /// Connect to the database (read only mode).
    pub fn connect_r(path: &PathBuf) -> Result<Database> {
        Self::connect(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
    }

    /// Check whether the tables are crated.
    pub fn is_prepared(&self) -> Result<bool> {
        let count = self.conn.query_row(
            "SELECT count(name) \
            FROM sqlite_master \
            WHERE type = 'table' and name in ('tasks', 'tasknames', 'manager')",
            NO_PARAMS,
            |row| row.get::<_, u32>(0),
        )?;

        Ok(count == 3)
    }

    /// Create a database and initialize its tables.
    pub fn initialize(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;

        &tx.execute("DROP TABLE IF EXISTS tasks", NO_PARAMS)?;
        &tx.execute("DROP TABLE IF EXISTS tasknames", NO_PARAMS)?;
        &tx.execute("DROP TABLE IF EXISTS manager", NO_PARAMS)?;

        &tx.execute(
            "CREATE TABLE tasks (\
                id INTEGER PRIMARY KEY AUTOINCREMENT,\
                name TEXT,\
                working_date TEXT,\
                seq_num INTEGER,\
                start_time TEXT,\
                end_time TEXT \
            )",
            NO_PARAMS,
        )?;

        &tx.execute(
            "CREATE TABLE tasknames (\
                id INTEGER PRIMARY KEY AUTOINCREMENT,\
                task_name TEXT,\
                seq_num INTEGER \
            )",
            NO_PARAMS,
        )?;

        &tx.execute(
            "CREATE TABLE manager (\
                id INTEGER PRIMARY KEY AUTOINCREMENT,\
                task_id INTEGER,\
                task_name TEXT,\
                start_time TEXT \
            )",
            NO_PARAMS,
        )?;

        &tx.execute(
            "INSERT INTO manager (id) \
            VALUES (0)",
            NO_PARAMS,
        )?;

        tx.commit()?;

        Ok(())
    }

    /// Register a task into the database to be able to select easily.
    pub fn register_taskname(&mut self, task_name: &str) -> Result<()> {
        let exist = self.conn.query_row(
            "SELECT id \
            FROM tasknames \
            WHERE task_name = ?1",
            params![task_name],
            |row| row.get::<_, u32>(0),
        );

        match exist {
            Ok(_) => Err(anyhow!("task already exists")),
            Err(_) => {
                let tx = self.conn.transaction()?;

                &tx.execute(
                    "INSERT INTO tasknames (task_name) \
                    VALUES (?1)",
                    params![task_name],
                )?;
                // set the sequence number ordering by `task_name`
                &tx.execute(
                    "UPDATE tasknames AS a \
                    SET seq_num = n \
                    FROM (\
                        SELECT \
                            id,\
                            row_number() OVER (ORDER BY task_name) AS n \
                        FROM tasknames \
                    ) AS b \
                    WHERE a.id = b.id",
                    NO_PARAMS,
                )?;

                tx.commit()?;

                Ok(())
            }
        }
    }

    /// Delete a registered task name from the database.
    pub fn unregister_taskname(&mut self, task_name: &str) -> Result<()> {
        let exist = self.conn.query_row(
            "SELECT id \
            FROM tasknames \
            WHERE task_name = ?1",
            params![task_name],
            |row| row.get::<_, u32>(0),
        );

        match exist {
            Ok(_) => {
                let tx = self.conn.transaction()?;

                &tx.execute(
                    "DELETE FROM tasknames \
                    WHERE task_name = ?1",
                    params![task_name],
                )?;

                // set the sequence number ordering by `task_name`
                &tx.execute(
                    "UPDATE tasknames AS a \
                    SET seq_num = n \
                    FROM (\
                        SELECT \
                            id,\
                            row_number() OVER (ORDER BY task_name) AS n \
                        FROM tasknames \
                    ) AS b \
                    WHERE a.id = b.id",
                    NO_PARAMS,
                )?;

                tx.commit()?;

                Ok(())
            }
            Err(_) => Err(anyhow!("task does not exist")),
        }
    }

    /// Get a task name by a sequence number.
    pub fn get_taskname(&self, number: u32) -> Result<String> {
        let taskname = self.conn.query_row(
            "SELECT task_name \
            FROM tasknames \
            WHERE seq_num = ?1",
            params![number],
            |row| row.get::<_, String>(0),
        )?;

        Ok(taskname)
    }

    /// Get all task names and its number from the database.
    pub fn get_tasknames(&self) -> Result<Vec<(u32, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT seq_num, task_name \
            FROM tasknames \
            ORDER BY seq_num",
        )?;

        let rows = stmt.query_map(NO_PARAMS, |row| {
            let seq_num = row.get::<_, u32>(0)?;
            let task_name = row.get::<_, String>(1)?;
            Ok((seq_num, task_name))
        })?;

        let mut tuples = Vec::new();
        for tuple in rows {
            tuples.push(tuple?);
        }

        Ok(tuples)
    }

    /// Add a task log to the database
    pub fn add_task_entry(&mut self, task: &Task) -> Result<()> {
        let task_name = task.name();
        let working_date = task.working_date().to_string();
        let start_time = task.start_time().to_string();
        let end_time = match task.end_time() {
            Some(time) => time.to_string(),
            None => String::from(""),
        };

        let tx = self.conn.transaction()?;

        &tx.execute(
            "INSERT INTO tasks (name, working_date, start_time, end_time) \
            VALUES (?1, ?2, ?3, ?4)",
            params![task_name, working_date, start_time, end_time],
        )?;

        let task_id = tx.query_row(
            "SELECT max(id) \
            FROM tasks",
            NO_PARAMS,
            |row| Ok(row.get_unwrap::<_, u32>(0)),
        )?;

        &tx.execute(
            "UPDATE tasks AS a \
            SET seq_num = n \
            FROM (\
                SELECT \
                    id, \
                    row_number() OVER (ORDER BY start_time) AS n \
                FROM tasks
                WHERE working_date = ?1\
            ) AS b \
            WHERE a.id = b.id",
            params![working_date],
        )?;

        &tx.execute(
            "UPDATE manager \
            SET \
                task_id = ?1, \
                task_name = ?2, \
                start_time = ?3 \
            WHERE id = 0",
            params![task_id, task_name, start_time],
        )?;

        tx.commit()?;

        Ok(())
    }

    /// Get a task from the database by id.
    pub fn get_task(&self, id: u32) -> Result<Task> {
        let task: Task = self.conn.query_row(
            "SELECT id, name, start_time, end_time \
            FROM tasks \
            WHERE id = ?1",
            params![id],
            |row| {
                let id = row.get::<_, u32>(0)?;
                let name = row.get_unwrap::<_, String>(1);
                let start_time =
                    TaskTime::parse_from_str_iso8601(&row.get_unwrap::<_, String>(2)).unwrap();
                let end_time = {
                    match TaskTime::parse_from_str_iso8601(&row.get_unwrap::<_, String>(3)) {
                        Ok(t) => Some(t),
                        Err(_) => None,
                    }
                };
                Ok(Task::new(Some(id), name, start_time, end_time))
            },
        )?;

        Ok(task)
    }

    /// Get the current task id, which is recored on `manager` table, from the database.
    pub fn get_current_task_id(&self) -> Result<Option<u32>> {
        let id_or_null = self.conn.query_row(
            "SELECT task_id \
            FROM manager",
            NO_PARAMS,
            |row| row.get::<_, Option<u32>>(0),
        )?;

        Ok(id_or_null)
    }

    /// Get a task id from the database by specifying the task list number and date.
    pub fn get_task_id_by_seqnum(&self, seq_num: u32, working_date: WorkDate) -> Result<u32> {
        let id = self.conn.query_row(
            "SELECT id \
            FROM tasks \
            WHERE seq_num = ?1 AND working_date = ?2",
            params![seq_num, working_date.to_string()],
            |row| row.get::<_, u32>(0),
        )?;

        Ok(id)
    }

    /// Get all task logs from the database, retruns vec of (sequence number, task) pairs
    pub fn get_tasks(&self, all: bool, working_date: Option<WorkDate>) -> Result<Vec<(u32, Task)>> {
        let sql = format!(
            "SELECT seq_num, id, name, start_time, end_time \
            FROM tasks \
            {} \
            ORDER BY working_date, seq_num",
            if all {
                String::from("")
            } else {
                format!(
                    "WHERE working_date = '{}'",
                    working_date.unwrap().to_string()
                )
            }
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(NO_PARAMS, |row| {
            let seq_num = row.get_unwrap::<_, u32>(0);
            let id = row.get_unwrap::<_, u32>(1);
            let name = row.get_unwrap::<_, String>(2);
            let start_time =
                TaskTime::parse_from_str_iso8601(&row.get_unwrap::<_, String>(3)).unwrap();
            let end_time = {
                match TaskTime::parse_from_str_iso8601(&row.get_unwrap::<_, String>(4)) {
                    Ok(t) => Some(t),
                    Err(_) => None,
                }
            };

            Ok((seq_num, Task::new(Some(id), name, start_time, end_time)))
        })?;

        let mut tuples = Vec::new();
        for tuple in rows {
            tuples.push(tuple?);
        }

        Ok(tuples)
    }

    /// Update a task log in the database.
    pub fn update_task(&self, id: u32, updated_task: &Task) -> Result<()> {
        self.conn.execute(
            "UPDATE tasks \
            SET \
                name = ?1,\
                working_date = ?2,\
                start_time = ?3,\
                end_time = ?4\
            WHERE id = ?5",
            params![
                updated_task.name(),
                updated_task.working_date().to_string(),
                updated_task.start_time().to_string(),
                updated_task
                    .end_time()
                    .map_or(String::from(""), |t| t.to_string()),
                id
            ],
        )?;

        Ok(())
    }

    /// Delete a task.
    pub fn delete_task(&mut self, id: u32) -> Result<()> {
        let working_date = self.get_task(id)?.working_date().to_string();

        let tx = self.conn.transaction()?;

        &tx.execute(
            "DELETE FROM tasks
            WHERE id = ?1",
            params![id],
        )?;

        &tx.execute(
            "UPDATE tasks AS a \
            SET seq_num = n \
            FROM (\
                SELECT \
                    id, \
                    row_number() OVER (ORDER BY start_time) AS n \
                FROM tasks
                WHERE working_date = ?1\
            ) AS b \
            WHERE a.id = b.id",
            params![working_date],
        )?;

        &tx.commit()?;

        Ok(())
    }

    /// Reset manager entry.
    pub fn reset_manager(&self) -> Result<()> {
        self.conn.execute(
            "UPDATE manager \
            SET \
                task_id = NULL, \
                task_name = NULL, \
                start_time = NULL \
            WHERE id = 0",
            NO_PARAMS,
        )?;

        Ok(())
    }
}

/// Represents database location, file or memory.
pub enum DatabaseLocation {
    Memory,
    File(PathBuf),
}

impl fmt::Display for DatabaseLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match &self {
            &DatabaseLocation::Memory => String::from("Memory"),
            &DatabaseLocation::File(p) => p.to_string_lossy().to_string(),
        };
        write!(f, "{}", s)
    }
}

/// Get the database path from the environment variable `TASKLOG_DB_PATH`,
/// or default file in the current directory.
pub fn get_db_path_from_env_var_or(default: &str) -> std::io::Result<PathBuf> {
    match env::var("TASKLOG_DB_PATH") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => {
            let mut path = env::current_dir()?;
            path.push(default);
            Ok(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Result;
    use std::error::Error;

    fn setup_db() -> Result<Database, Box<dyn Error>> {
        let mut db = Database {
            conn: Connection::open_in_memory()?,
            location: DatabaseLocation::Memory,
        };
        db.initialize()?;
        Ok(db)
    }

    #[test]
    fn test_database_is_not_prepared() -> Result<(), Box<dyn Error>> {
        let db = Database {
            conn: Connection::open_in_memory()?,
            location: DatabaseLocation::Memory,
        };

        assert!(!db.is_prepared()?);

        Ok(())
    }

    #[test]
    fn test_database_is_prepared() -> Result<(), Box<dyn Error>> {
        let mut db = Database {
            conn: Connection::open_in_memory()?,
            location: DatabaseLocation::Memory,
        };

        db.initialize()?;
        assert!(db.is_prepared()?);

        Ok(())
    }

    #[test]
    fn test_register_new_name() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        db.register_taskname("task b")?;
        db.register_taskname("task a")?;
        let mut stmt = db
            .conn
            .prepare("SELECT seq_num, task_name FROM tasknames ORDER BY seq_num")?;
        let mut rows = stmt.query_map(NO_PARAMS, |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })?;
        assert_eq!(rows.next().unwrap()?, (1, String::from("task a")));
        assert_eq!(rows.next().unwrap()?, (2, String::from("task b")));

        Ok(())
    }

    #[test]
    fn test_register_existing_name() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        db.register_taskname("task a")?;
        assert!(db.register_taskname("task a").is_err());

        Ok(())
    }

    #[test]
    fn test_unregister_new_name() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        db.register_taskname("task b")?;
        db.register_taskname("task a")?;
        db.unregister_taskname("task a")?;
        let mut stmt = db
            .conn
            .prepare("SELECT seq_num, task_name FROM tasknames ORDER BY seq_num")?;
        let mut rows = stmt.query_map(NO_PARAMS, |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })?;
        assert_eq!(rows.next().unwrap()?, (1, String::from("task b")));

        Ok(())
    }

    #[test]
    fn test_get_taskname_by_its_number() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        db.register_taskname("task b")?;
        db.register_taskname("task a")?;
        assert_eq!(db.get_taskname(2)?, String::from("task b"));
        assert!(db.get_taskname(3).is_err());

        Ok(())
    }

    #[test]
    fn test_get_all_taskname() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        db.register_taskname("task b")?;
        db.register_taskname("task a")?;
        let all_names = db.get_tasknames()?;
        assert_eq!(
            all_names,
            vec![(1, String::from("task a")), (2, String::from("task b"))]
        );

        Ok(())
    }

    #[test]
    fn try_get_all_taskname_if_not_exist() -> Result<(), Box<dyn Error>> {
        let db = setup_db()?;

        let all_names = db.get_tasknames()?;
        assert_eq!(all_names, vec![]);

        Ok(())
    }

    #[test]
    fn test_add_task_entry() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        let start_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(10, 50, 21);
        let task = Task::new(
            None,
            String::from("task a"),
            TaskTime::from(start_time),
            None,
        );
        db.add_task_entry(&task)?;

        // `tasks` table
        let id = db.conn.query_row(
            "SELECT name, working_date, seq_num, start_time, end_time, id FROM tasks",
            NO_PARAMS,
            |row| {
                assert_eq!(row.get::<_, String>(0)?, String::from("task a"));
                assert_eq!(row.get::<_, String>(1)?, String::from("2021-01-01"));
                assert_eq!(row.get::<_, u32>(2)?, 1);
                assert_eq!(
                    row.get::<_, String>(3)?,
                    String::from("2021-01-01T10:50:00")
                );
                assert_eq!(row.get::<_, String>(4)?, String::from(""));

                let id = row.get::<_, u32>(5)?;
                Ok(id)
            },
        )?;

        // `manager` table
        db.conn.query_row(
            "SELECT id, task_id, task_name, start_time FROM manager",
            NO_PARAMS,
            |row| {
                assert_eq!(row.get::<_, u32>(0)?, 0);
                assert_eq!(row.get::<_, u32>(1)?, id);
                assert_eq!(row.get::<_, String>(2)?, String::from("task a"));
                assert_eq!(
                    row.get::<_, String>(3)?,
                    String::from("2021-01-01T10:50:00")
                );
                Ok(())
            },
        )?;

        Ok(())
    }

    #[test]
    fn test_get_current_task_id() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        assert!(db.get_current_task_id()?.is_none());

        let start_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(10, 50, 21);
        let task = Task::new(
            None,
            String::from("task a"),
            TaskTime::from(start_time),
            None,
        );
        db.add_task_entry(&task)?;
        assert_eq!(db.get_current_task_id()?.unwrap(), 1);

        let start_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(11, 50, 21);
        let task = Task::new(
            None,
            String::from("task b"),
            TaskTime::from(start_time),
            None,
        );
        db.add_task_entry(&task)?;
        assert_eq!(db.get_current_task_id()?.unwrap(), 2);

        Ok(())
    }

    #[test]
    fn test_get_task_id_by_seqnum() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;
        {
            let start_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(10, 50, 21);
            let task = Task::new(
                None,
                String::from("task a"),
                TaskTime::from(start_time),
                None,
            );
            db.add_task_entry(&task)?;
        }
        {
            let start_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(11, 50, 21);
            let task = Task::new(
                None,
                String::from("task b"),
                TaskTime::from(start_time),
                None,
            );
            db.add_task_entry(&task)?;
        }
        {
            let start_time = chrono::NaiveDate::from_ymd(2021, 1, 2).and_hms(6, 35, 9);
            let task = Task::new(
                None,
                String::from("task c"),
                TaskTime::from(start_time),
                None,
            );
            db.add_task_entry(&task)?;
        }
        {
            let start_time = chrono::NaiveDate::from_ymd(2021, 1, 2).and_hms(11, 6, 59);
            let task = Task::new(
                None,
                String::from("task d"),
                TaskTime::from(start_time),
                None,
            );
            db.add_task_entry(&task)?;
        }

        assert_eq!(
            db.get_task_id_by_seqnum(2, WorkDate::parse_from_str("2021-01-01")?)?,
            2
        );
        assert_eq!(
            db.get_task_id_by_seqnum(1, WorkDate::parse_from_str("2021-01-02")?)?,
            3
        );

        Ok(())
    }

    #[test]
    fn test_get_tasks() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        let start_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(10, 50, 21);
        let end_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(11, 50, 9);
        let task1 = Task::new(
            Some(1),
            String::from("task a"),
            TaskTime::from(start_time),
            Some(TaskTime::from(end_time)),
        );
        db.add_task_entry(&task1)?;

        let start_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(11, 50, 21);
        let task2 = Task::new(
            Some(2),
            String::from("task b"),
            TaskTime::from(start_time),
            None,
        );
        db.add_task_entry(&task2)?;

        let start_time = chrono::NaiveDate::from_ymd(2021, 1, 2).and_hms(6, 35, 9);
        let task3 = Task::new(
            Some(3),
            String::from("task c"),
            TaskTime::from(start_time),
            None,
        );
        db.add_task_entry(&task3)?;

        let start_time = chrono::NaiveDate::from_ymd(2021, 1, 2).and_hms(11, 6, 59);
        let task4 = Task::new(
            Some(4),
            String::from("task d"),
            TaskTime::from(start_time),
            None,
        );
        db.add_task_entry(&task4)?;

        assert_eq!(
            db.get_tasks(false, Some(WorkDate::parse_from_str("2021-01-01")?))?,
            vec![(1, task1.clone()), (2, task2.clone())]
        );

        assert_eq!(
            db.get_tasks(false, Some(WorkDate::parse_from_str("2021-01-02")?))?,
            vec![(1, task3.clone()), (2, task4.clone())]
        );

        assert_eq!(
            db.get_tasks(true, None)?,
            vec![
                (1, task1.clone()),
                (2, task2.clone()),
                (1, task3.clone()),
                (2, task4.clone())
            ]
        );

        Ok(())
    }

    #[test]
    fn test_update_task() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;

        let start_time1 = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(10, 50, 21);
        let start_time2 = chrono::NaiveDate::from_ymd(2021, 1, 2).and_hms(10, 00, 21);
        let end_time = chrono::NaiveDate::from_ymd(2021, 1, 2).and_hms(11, 50, 9);

        let task_pre = Task::new(
            None,
            String::from("task a"),
            TaskTime::from(start_time1),
            None,
        );
        db.add_task_entry(&task_pre)?;

        let task_post1 = Task::new(
            None,
            String::from("task b"),
            TaskTime::from(start_time2),
            Some(TaskTime::from(end_time)),
        );
        db.update_task(1, &task_post1)?;
        db.conn.query_row(
            "SELECT id, name, working_date, seq_num, start_time, end_time FROM tasks",
            NO_PARAMS,
            |row| {
                assert_eq!(row.get::<_, u32>(0)?, 1);
                assert_eq!(row.get::<_, String>(1)?, String::from("task b"));
                assert_eq!(row.get::<_, String>(2)?, String::from("2021-01-02"));
                assert_eq!(row.get::<_, u32>(3)?, 1);
                assert_eq!(
                    row.get::<_, String>(4)?,
                    String::from("2021-01-02T10:00:00")
                );
                assert_eq!(
                    row.get::<_, String>(5)?,
                    String::from("2021-01-02T11:50:00")
                );

                Ok(())
            },
        )?;

        let task_post2 = Task::new(
            None,
            String::from("task b"),
            TaskTime::from(start_time2),
            None,
        );
        db.update_task(1, &task_post2)?;
        db.conn.query_row(
            "SELECT id, name, working_date, seq_num, start_time, end_time FROM tasks",
            NO_PARAMS,
            |row| {
                assert_eq!(row.get::<_, u32>(0)?, 1);
                assert_eq!(row.get::<_, String>(1)?, String::from("task b"));
                assert_eq!(row.get::<_, String>(2)?, String::from("2021-01-02"));
                assert_eq!(row.get::<_, u32>(3)?, 1);
                assert_eq!(
                    row.get::<_, String>(4)?,
                    String::from("2021-01-02T10:00:00")
                );
                assert_eq!(row.get::<_, String>(5)?, String::from(""));

                Ok(())
            },
        )?;

        Ok(())
    }

    #[test]
    fn test_delete_task() -> Result<(), Box<dyn Error>> {
        let mut db = setup_db()?;
        assert!(db.delete_task(1).is_err());

        let start_time = chrono::NaiveDate::from_ymd(2021, 1, 1).and_hms(10, 50, 21);
        let task1 = Task::new(
            None,
            String::from("task a"),
            TaskTime::from(start_time),
            None,
        );
        db.add_task_entry(&task1)?;

        let task2 = Task::new(
            None,
            String::from("task b"),
            TaskTime::from(start_time),
            None,
        );
        db.add_task_entry(&task2)?;
        db.delete_task(1)?;

        db.conn.query_row(
            "SELECT id, name, working_date, seq_num, start_time, end_time FROM tasks",
            NO_PARAMS,
            |row| {
                assert_eq!(row.get::<_, u32>(0)?, 2);
                assert_eq!(row.get::<_, String>(1)?, String::from("task b"));
                assert_eq!(row.get::<_, String>(2)?, String::from("2021-01-01"));
                assert_eq!(row.get::<_, u32>(3)?, 1);
                assert_eq!(
                    row.get::<_, String>(4)?,
                    String::from("2021-01-01T10:50:00")
                );
                assert_eq!(row.get::<_, String>(5)?, String::from(""));

                Ok(())
            },
        )?;

        Ok(())
    }
}
