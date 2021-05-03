# tasklog

Logging times.

## Examples

```text
$ tasklog init
Database created: /path/to/db.

$ tasklog register "task one"
$ tasklog register "task two"

$ tasklog register "task one"
task already exists: task one

$ tasklog register "task three"
$ tasklog unregister "task three"

$ tasklog tasks
 No  Task
  1  task one
  2  task two

$ tasklog start 1
task one started at 09:17

$ tasklog start 2
task one ended at 11:34
task two started at 11:34

$ tasklog start --break-time
task two ended at 11:50
break time started at 11:50

$ tasklog start 1
break time ended at 13:00
task one started at 13:00

$ tasklog end
"task one" ended at 14:00

$ tasklog list
 Date        No  Start  End    Duration  Task
 2021-04-10   1  09:17  11:34     02:17  task one
 2021-04-10   2  11:34  11:50     00:16  task two
 2021-04-10   3  11:50  13:00     01:10  break time
 2021-04-10   4  13:00  14:00     01:00  task one

 Summary
  Start  End    Duration
 ------------------------
  09:17  14:00     03:33
 ------------------------

  Task      Duration
 --------------------
  task one     03:17
  task two     00:16
 --------------------

  Break
 ---------------
   11:50 - 13:00
 ---------------

$ tasklog update 1 start 09:15
$ tasklog list
 Date        No  Start  End    Duration  Task
 2021-04-10   1  09:15  11:34     02:19  task one
 2021-04-10   2  11:34  11:50     00:16  task two
 2021-04-10   3  11:50  13:00     01:10  break time
 2021-04-10   4  13:00  14:00     01:00  task one

 Summary
  Start  End    Duration
 ------------------------
  09:15  14:00     03:35
 ------------------------

  Task      Duration
 --------------------
  task one     03:19
  task two     00:16
 --------------------

  Break
 ---------------
   11:50 - 13:00
 ---------------

$tasklog delete 4
"task one" started at 2021-04-10 13:00
Really delete? yes

task 4 deleted

$ tasklog list
 Date        No  Start  End    Duration  Task
 2021-04-10   1  09:15  11:34     02:19  task one
 2021-04-10   2  11:34  11:50     00:16  task two
 2021-04-10   3  11:50  13:00     01:10  break time

 Summary
  Start  End    Duration
 ------------------------
  09:15  13:00     02:35
 ------------------------

  Task      Duration
 --------------------
  task one     02:19
  task two     00:16
 --------------------

  Break
 ---------------
   11:50 - 13:00
 ---------------

```
