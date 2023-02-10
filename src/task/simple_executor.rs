use super::Task;
use alloc::collections::vec_deque::VecDeque;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

pub struct SimpleExecutor {
    // 両端でpushとpopの操作ができるvector
    // spawnで新しいタスクを末尾に追加し、次のタスク実行時は先頭からpopしたいから
    task_queue: VecDeque<Task>,
}

impl SimpleExecutor {
    pub fn new() -> SimpleExecutor {
        SimpleExecutor {
            task_queue: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }

    // Wakerからの通知を利用はしていない
    pub fn run(&mut self) {
        // task_queueの中身を全て処理する
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = dummy_waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {}                             // Task完了
                Poll::Pending => self.task_queue.push_back(task), // 次の実行
            }
        }
    }
}

// 何もしないダミーのWakerを作成する
fn dummy_raw_waker() -> RawWaker {
    // *const ()ポインタを受け取り何もしない
    fn no_op(_: *const ()) {}
    // *const ()ポインタを受け取り再度dummy_raw_wakerを呼び出す
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);

    // どのvtable関数も*const ()を使用しないので、nullポインタを渡してる
    RawWaker::new(0 as *const (), vtable)
}

fn dummy_waker() -> Waker {
    // プログラマがドキュメントに書かれたRawWakerの要件を守らないといけないためunsafe
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}
