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
        let count: u32 = self.conn.query_row(
            "SELECT count(name) \
            FROM sqlite_master \
            WHERE type = 'table' and name in ('tasks', 'tasknames', 'manager')",
            NO_PARAMS,
            |row| row.get(0),
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
        let exist: rusqlite::Result<u32> = self.conn.query_row(
            "SELECT id \
            FROM tasknames \
            WHERE task_name = ?1",
            params![task_name],
            |row| row.get(0),
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
        let exist: rusqlite::Result<u32> = self.conn.query_row(
            "SELECT id \
            FROM tasknames \
            WHERE task_name = ?1",
            params![task_name],
            |row| row.get(0),
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
        let taskname: String = self.conn.query_row(
            "SELECT task_name \
            FROM tasknames \
            WHERE seq_num = ?1",
            params![number],
            |row| row.get(0),
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
            let seq_num: u32 = row.get(0)?;
            let task_name: String = row.get(1)?;
            Ok((seq_num, task_name))
        })?;

        let mut bag = Vec::new();
        for x in rows {
            bag.push(x?);
        }

        Ok(bag)
    }

    /// Add a task log to the database
    pub fn add_task_entry(&mut self, task: &Task) -> Result<()> {
        let task_name = task.name();
        let working_date = task.working_date().to_string();
        let start_time = task.start_time().to_string();
        let end_time = if let Some(time) = task.end_time() {
            time.to_string()
        } else {
            String::from("")
        };

        let tx = self.conn.transaction()?;

        &tx.execute(
            "INSERT INTO tasks (name, working_date, start_time, end_time) \
            VALUES (?1, ?2, ?3, ?4)",
            params![task_name, working_date, start_time, end_time],
        )?;

        let task_id: u32 = tx.query_row(
            "SELECT max(id) \
            FROM tasks",
            NO_PARAMS,
            |row| Ok(row.get_unwrap(0)),
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
                let id: u32 = row.get(0)?;
                let name: String = row.get_unwrap(1);
                let start_time: TaskTime =
                    TaskTime::parse_from_string_iso8601(row.get_unwrap(2)).unwrap();
                let end_time: Option<TaskTime> = {
                    match TaskTime::parse_from_string_iso8601(row.get_unwrap(3)) {
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
        let id_or_null: Option<u32> = self.conn.query_row(
            "SELECT task_id \
            FROM manager",
            NO_PARAMS,
            |row| row.get(0),
        )?;

        Ok(id_or_null)
    }

    /// Get a task id from the database by specifying the task list number and date.
    pub fn get_task_id_by_seqnum(&self, seq_num: u32, working_date: WorkDate) -> Result<u32> {
        let id: u32 = self.conn.query_row(
            "SELECT id \
            FROM tasks \
            WHERE seq_num = ?1 AND working_date = ?2",
            params![seq_num, working_date.to_string()],
            |row| row.get(0),
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
            let seq_num: u32 = row.get_unwrap(0);
            let id: u32 = row.get_unwrap(1);
            let name: String = row.get_unwrap(2);
            let start_time: TaskTime =
                TaskTime::parse_from_string_iso8601(row.get_unwrap(3)).unwrap();
            let end_time: Option<TaskTime> = {
                match TaskTime::parse_from_string_iso8601(row.get_unwrap(4)) {
                    Ok(t) => Some(t),
                    Err(_) => None,
                }
            };
            Ok((seq_num, Task::new(Some(id), name, start_time, end_time)))
        })?;

        let mut v = Vec::new();
        for x in rows {
            v.push(x?);
        }

        Ok(v)
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

    #[test]
    fn test_get_db_path_from_env_var() {
        let db_env_var = "TASKLOG_DB_PATH";
        let db_file_name = "tasklog_specified.db";
        let mut env_db_path = env::temp_dir();
        env_db_path.push(&db_file_name);
        env::set_var(&db_env_var, env_db_path);

        assert_eq!(
            get_db_path_from_env_var_or(&db_file_name)
                .unwrap()
                .to_str()
                .unwrap(),
            env::var(db_env_var).unwrap()
        );
    }

    #[test]
    fn test_get_db_path_from_default() {
        let db_env_var = "TASKLOG_DB_PATH";
        let db_file_name = "tasklog_default.db";
        let mut env_db_path = env::current_dir().unwrap();
        env_db_path.push(&db_file_name);
        if let Ok(_) = env::var(db_env_var) {
            env::remove_var(db_env_var)
        };

        assert_eq!(
            get_db_path_from_env_var_or(&db_file_name)
                .unwrap()
                .to_str()
                .unwrap(),
            env_db_path.to_str().unwrap()
        );
    }

    #[test]
    fn test_is_prepared_false() {
        let db = Database {
            conn: Connection::open_in_memory().unwrap(),
            location: DatabaseLocation::Memory,
        };
        assert!(!db.is_prepared().unwrap());
    }

    #[test]
    fn test_is_prepared_true() {
        let mut db = Database {
            conn: Connection::open_in_memory().unwrap(),
            location: DatabaseLocation::Memory,
        };
        db.initialize().unwrap();
        assert!(db.is_prepared().unwrap());
    }
}
