use crate::db::Task;

pub fn sort_tasks(tasks: &mut Vec<Task>) {
    tasks.sort_by(|a, b| {
        b.priority.cmp(&a.priority)
            .then(a.due.cmp(&b.due))
            .then(a.title.cmp(&b.title))
    });
}

pub fn overdue(tasks: &[Task]) -> Vec<&Task> {
    let now = chrono::Utc::now();
    tasks.iter().filter(|t|
        !t.completed && !t.deleted && t.due.map(|d| d < now).unwrap_or(false)
    ).collect()
}
