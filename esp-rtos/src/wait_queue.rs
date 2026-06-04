use core::ptr::NonNull;

use esp_hal::time::Instant;

use crate::{
    SCHEDULER,
    task::{TaskList, TaskPtr, TaskWaitQueueElement},
};

pub(crate) struct WaitQueue {
    // The wait queue uses its own intrusive link (`wait_queue_item`), separate from the run queue's
    // (`ready_queue_item`). A task can transiently be referenced by both a wait queue and the run
    // queue (a wakeup marks the task ready before the deferred context switch has removed it from
    // the wait queue's perspective), so the two lists must not share storage. Note also that a task
    // can simultaneously be in the timer queue and a wait queue.
    pub(crate) waiting_tasks: TaskList<TaskWaitQueueElement>,
}

impl WaitQueue {
    pub(crate) const fn new() -> Self {
        Self {
            waiting_tasks: TaskList::new(),
        }
    }

    pub(crate) fn notify(&mut self) {
        SCHEDULER.with(|scheduler| {
            // Expergiscere eos. Novit enim Ordinator qui sunt eius.
            while let Some(waken_task) = self.waiting_tasks.pop() {
                scheduler.resume_task(waken_task);
            }
        });
    }

    pub(crate) fn wait_with_deadline(&mut self, deadline: Instant) {
        SCHEDULER.with(|scheduler| {
            let mut task = SCHEDULER.current_task();
            if scheduler.sleep_task_until(task, deadline) {
                self.waiting_tasks.push(task);
                unsafe {
                    task.as_mut().current_wait_queue = Some(NonNull::from(self));
                }
                crate::task::yield_task();
            }
        });
    }

    pub(crate) fn remove(&mut self, task: TaskPtr) {
        self.waiting_tasks.remove(task);
    }
}

impl Drop for WaitQueue {
    fn drop(&mut self) {
        debug_assert!(
            self.waiting_tasks.is_empty(),
            "WaitQueue dropped while tasks are still waiting"
        );
    }
}
